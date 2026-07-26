use std::{
    ops::Deref,
    sync::{Arc, Mutex, atomic::AtomicUsize},
    time::Duration,
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, future::try_join_all, stream};
use indexmap::{IndexMap, IndexSet};
use m3u8_rs::{
    MasterPlaylist, MediaPlaylist, VariantStream, parse_master_playlist, parse_media_playlist,
};
use nom::Finish;
use tokio::{
    fs::{self},
    io::AsyncWriteExt,
    time,
};
use url::Url;
use wreq::header::REFERER;

use crate::{
    Episode,
    anime::episode_detail::{EpisodeDetail, EpisodeDetailBuildError},
    config::Config,
    constant::{ORIGIN, TMP_DIR_PATH},
    device_id::DeviceId,
    ffmpeg::{FFmpeg, FFmpegError},
    request::{
        self, RequestClient,
        common::DirectDataResponseBody,
        playlist::PlaylistSrc,
        token::{Token, TokenError},
    },
    util::{get_referer, random_string},
};

// TODO listen cookie jar change, save to config file
pub struct EpisodeDownloader {
    request_client: Arc<RequestClient>,
    config: Arc<Mutex<Config>>,
    device_id: DeviceId,
}

#[derive(Debug, thiserror::Error)]
pub enum EpisodeDownloadError {
    #[error("解析 URL 失败: {0}")]
    Url(#[from] url::ParseError),

    #[error("HTTP 请求失败: {0}")]
    Http(#[from] wreq::Error),

    #[error("文件系统操作失败: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFmpeg 执行失败: {0}")]
    Ffmpeg(#[from] FFmpegError),

    #[error("读取剧集信息失败: {0}")]
    EpisodeDetail(#[from] EpisodeDetailBuildError),

    #[error("用户权限校验失败: {0}")]
    Permission(#[from] TokenError),

    #[error("解析播放列表失败: {0}")]
    M3u8Parse(nom::error::Error<String>),

    #[error("设备 ID 不存在")]
    DeviceIdMissing,

    #[error("已设置仅限 VIP，但当前账号不是 VIP")]
    VipRequired,

    #[error("等待广告结束超时")]
    AdsTimeout,

    #[error("没有符合要求的分辨率，可用分辨率: {available:?}")]
    ResolutionUnavailable { available: Vec<u64> },

    #[error("动画疯接口返回错误: {0}")]
    Api(String),

    #[error("路径解析错误：{0}")]
    Path(String),

    #[error("播放清单解析错误：{0}")]
    PlaylistParseError(String),
}

impl From<nom::error::Error<&[u8]>> for EpisodeDownloadError {
    fn from(value: nom::error::Error<&[u8]>) -> Self {
        Self::M3u8Parse(nom::error::Error::new(
            String::from_utf8_lossy(value.input).into_owned(),
            value.code,
        ))
    }
}

pub type EpisodeDownloadResult<T = ()> = Result<T, EpisodeDownloadError>;

impl EpisodeDownloader {
    pub fn new(
        request_client: Arc<RequestClient>,
        config: Arc<Mutex<Config>>,
        device_id: DeviceId,
    ) -> Self {
        Self {
            request_client,
            config,
            device_id,
        }
    }
    pub async fn download(&self, episode: &Episode) -> EpisodeDownloadResult {
        let (sn, episode, cover) = episode.clone().into_parts();
        let inner_downloader = InnerDownloader {
            sn,
            cover,
            episode,

            request_client: self.request_client.clone(),
            config: self.config.clone(),
            device_id: self.device_id.clone(),
        };

        inner_downloader.download().await?;

        Ok(())
    }
}

struct InnerDownloader {
    request_client: Arc<RequestClient>,
    config: Arc<Mutex<Config>>,
    device_id: DeviceId,

    cover: String,
    episode: u32,
    sn: u32,
}

impl InnerDownloader {
    #[tracing::instrument(skip(self))]
    async fn download(&self) -> EpisodeDownloadResult {
        if !FFmpeg::exist().await? {
            return Err(FFmpegError::FFmpegNotExist.into());
        }

        self.get_device_id().await?;
        tracing::debug!("renew device_id complete");

        let token = self.gain_access(true).await?;
        tracing::debug!("gain access complete");

        self.unlock(None).await?;
        self.check_lock().await?;
        self.unlock(None).await?;
        self.unlock(None).await?;
        tracing::debug!("check lock complete");

        if !token.vip() {
            if self.config.lock().unwrap().only_use_vip {
                return Err(EpisodeDownloadError::VipRequired);
            }

            tracing::info!(
                self.sn,
                self.episode,
                "start waiting for ads because of not vip account"
            );

            self.start_ad().await?;
            tracing::debug!("start waiting ad");
            let ads_time = { self.config.lock().unwrap().ads_time as u64 };
            time::sleep(Duration::from_secs(ads_time)).await;
            tracing::debug!("finish wating, skip ad");
            self.skip_ad().await?;
            tracing::debug!("skip ad  complete");
        } else {
            tracing::info!("recognize vip account, start downloading");
        }

        self.video_start().await?;
        tracing::debug!("start video");
        self.check_no_ad().await?;
        tracing::debug!("check no ad complete");

        // FIXME it seems like that ani gamer doesn't use this api now. detailed api url is inside
        // this method
        let playlist = self.get_playlist().await?;

        let src = playlist.src();

        tracing::debug!(%src, "master playlist");

        let master_pl_bytes = self
            .request_client
            .get(src, false)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let (_, master_pl) = parse_master_playlist(&master_pl_bytes).finish()?;
        if master_pl.variants.is_empty() {
            return Err(EpisodeDownloadError::PlaylistParseError(
                "m3u8 资源单中没有任何清晰度".into(),
            ));
        }
        tracing::debug!("parsed master_pl");

        let episode_detail = EpisodeDetail::from_sn(self.sn, self.request_client.clone()).await?;
        episode_detail.ensure_bangumi_dir().await?;
        tracing::debug!("ensured bangumi_dir");

        let variant = self.get_media_variant(master_pl)?;
        tracing::debug!(
            resolution = variant.resolution.unwrap().height,
            "selected media variant"
        );

        let file_name = episode_detail.get_filename(
            variant
                .resolution
                .expect("it should have resolution")
                .height,
        );

        let file_path = episode_detail.bangumi_dir().join(&file_name);

        let tmp_dir_path = TMP_DIR_PATH.join(self.sn.to_string());
        fs::create_dir_all(&tmp_dir_path).await?;
        tracing::debug!(
            file_name = file_name,
            file_path = %file_path.display(),
            tmp_dir_path = %tmp_dir_path.display(),
            "decided path and file name"
        );

        let media_pl_src = Url::parse(src)?.join(&variant.uri)?;
        tracing::debug!(url = %media_pl_src, "parsed media url");
        let (media_pl_bytes, media_pl) = self.get_media_playlist(&media_pl_src).await?;
        tracing::debug!("parsed media playlist");

        // download all key file
        try_join_all(
            media_pl
                .segments
                .iter()
                .filter_map(|segment| segment.key.as_ref().and_then(|key| key.uri.to_owned()))
                .collect::<IndexSet<String>>()
                .into_iter()
                .map(|uri| async {
                    let uri = uri;
                    let mut resp = self
                        .request_client
                        .get(media_pl_src.join(&uri)?, false)
                        .header(REFERER, get_referer(self.sn))
                        .send()
                        .await?
                        .error_for_status()?;
                    let mut file = fs::File::create(tmp_dir_path.join(&uri)).await?;

                    while let Some(chunk) = resp.chunk().await? {
                        file.write_all(&chunk).await?;
                    }

                    file.flush().await?;

                    Ok::<(), EpisodeDownloadError>(())
                }),
        )
        .await?;
        tracing::debug!("downloaded keys");

        // write m3u8 playlist to file
        fs::write(tmp_dir_path.join("manifest.m3u8"), media_pl_bytes).await?;
        tracing::debug!("writed m3u8 manifest");

        let multi_downloading_segment = { self.config.lock().unwrap().multi_downloading_segment };
        let completed = Arc::new(AtomicUsize::new(0));
        let total = media_pl.segments.len();
        stream::iter(&media_pl.segments)
            .map(|segment| async {
                let mut resp = self
                    .request_client
                    .get(media_pl_src.join(segment.uri.deref())?, false)
                    .header(REFERER, get_referer(self.sn))
                    .send()
                    .await?
                    .error_for_status()?;

                let mut file = fs::File::create(tmp_dir_path.join(segment.uri.deref())).await?;

                while let Some(chunk) = resp.chunk().await? {
                    file.write_all(&chunk).await?
                }

                file.flush().await?;

                Ok::<(), EpisodeDownloadError>(())
            })
            .buffer_unordered(multi_downloading_segment)
            .inspect_ok(|_| {
                let current = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                tracing::debug!(current = current, total = total, "downloading");
            })
            .try_collect::<Vec<()>>()
            .await?;
        tracing::debug!("m3u8 chunks downloaded");

        FFmpeg::merge_m3u8(
            tmp_dir_path
                .join("manifest.m3u8")
                .to_str()
                .ok_or(EpisodeDownloadError::Path(
                    tmp_dir_path.display().to_string(),
                ))?,
            tmp_dir_path.join(&file_name).to_string_lossy().into_owned(),
        )
        .await?;
        tracing::debug!("merged m3u8 to mp4");

        fs::copy(tmp_dir_path.join(&file_name), file_path).await?;
        fs::remove_file(tmp_dir_path.join(&file_name)).await?;
        tracing::debug!("moved file");

        fs::remove_dir_all(tmp_dir_path).await?;
        tracing::debug!("removed tmp dir");

        Ok(())
    }

    async fn get_device_id(&self) -> EpisodeDownloadResult {
        let mut url = Url::parse(ORIGIN)?;
        url.set_path("ajax/getdeviceid.php");
        if let Some(device_id) = self.device_id.get_cloned() {
            url.set_query(Some(&format!("id={device_id}")));
        }
        let device_id: request::device_id::DeviceId = self
            .request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        self.device_id.set(Some(device_id.into_device_id()));

        Ok(())
    }

    /// if it is called before add, it should add adId, otherwise not
    async fn gain_access(&self, add_ad_id: bool) -> EpisodeDownloadResult<Token> {
        let mut url = Url::parse(ORIGIN)?;
        url.set_path("ajax/token.php");
        {
            let mut query_pairs = url.query_pairs_mut();
            if add_ad_id {
                query_pairs.append_pair("adId", "0");
            }
            query_pairs
                .append_pair("sn", self.sn.to_string().as_str())
                .append_pair(
                    "device",
                    self.device_id
                        .get_cloned()
                        .ok_or(EpisodeDownloadError::DeviceIdMissing)?
                        .as_str(),
                )
                .append_pair("hash", &random_string(12));
        };

        let token = self
            .request_client
            .get(url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?
            .json::<DirectDataResponseBody<Token, TokenError>>()
            .await?
            .into_result()?;
        // let token_text = self
        //     .request_client
        //     .get(url, true)
        //     .header(REFERER, get_referer(self.sn))
        //     .send()
        //     .await?
        //     .error_for_status()?
        //     .text()
        //     .await?;
        // println!("{token_text}");

        // let token = serde_json::from_str::<DirectDataResponseBody<Token, TokenError>>(&token_text)
        //     .map_err(|err| AnimeDownloadError::Plain(format!("parse json error: {err}")))?
        //     .into_result()?;

        Ok(token)
    }

    async fn unlock(&self, ttl: Option<u32>) -> EpisodeDownloadResult {
        let ttl = ttl.unwrap_or(0);

        let url = format!("{ORIGIN}/ajax/unlock.php?sn={}&ttl={ttl}", self.sn);
        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn check_lock(&self) -> EpisodeDownloadResult {
        let url = format!(
            "{ORIGIN}/ajax/checklock.php?device={}&sn={}",
            {
                self.device_id
                    .get_cloned()
                    .ok_or(EpisodeDownloadError::DeviceIdMissing)?
            },
            self.sn
        );

        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn start_ad(&self) -> EpisodeDownloadResult {
        // TODO s=194699 is ad's id, real logic will rotate this
        let url = format!("{ORIGIN}/ajax/videoCastcishu.php?sn={}&s=194699", self.sn);

        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn skip_ad(&self) -> EpisodeDownloadResult {
        let url = format!(
            "{ORIGIN}/ajax/videoCastcishu.php?sn={}&s=194699&ad=end",
            self.sn
        );

        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn video_start(&self) -> EpisodeDownloadResult {
        let url = format!("{ORIGIN}/ajax/videoStart.php?sn={}", self.sn);

        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn check_no_ad(&self) -> EpisodeDownloadResult {
        for _ in 0..10 {
            let token = self.gain_access(false).await?;

            if token.time() != 1 {
                time::sleep(Duration::from_secs(2)).await;
                self.skip_ad().await?;
                self.video_start().await?;
                continue;
            }

            // TODO modify ads_time in config, then write to file
            return Ok(());
        }

        Err(EpisodeDownloadError::AdsTimeout)
    }

    async fn get_playlist(&self) -> EpisodeDownloadResult<PlaylistSrc> {
        // FIXME it seems like using
        // https://api.gamer.com.tw/anime/v1/video_src.php?videoSn=49953&deviceid=0118ddf4664ceba5c04a12aec5025b4d7c93819911f881586a5a65f00630&deviceTypeUseCases=1
        // now
        let url = format!(
            "{ORIGIN}/ajax/m3u8.php?sn={}&device={}",
            self.sn,
            self.device_id
                .get_cloned()
                .ok_or(EpisodeDownloadError::DeviceIdMissing)?
        );

        let playlist = self
            .request_client
            .get(url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?
            .json::<DirectDataResponseBody<PlaylistSrc, String>>()
            .await?
            .into_result()
            .map_err(EpisodeDownloadError::Api)?;

        Ok(playlist)
    }

    fn get_media_variant(&self, master_pl: MasterPlaylist) -> EpisodeDownloadResult<VariantStream> {
        let resolution_map = master_pl
            .variants
            .into_iter()
            .map(|variant| {
                Ok::<(u64, VariantStream), EpisodeDownloadError>((
                    variant
                        .resolution
                        .ok_or(EpisodeDownloadError::PlaylistParseError(format!(
                            "存在无法解析清晰度的变体：{:?}",
                            variant
                        )))?
                        .height,
                    variant,
                ))
            })
            .collect::<EpisodeDownloadResult<IndexMap<u64, VariantStream>>>()?;

        let select_resolution = { self.config.lock().unwrap().download_resolution.get_height() };
        let variant = resolution_map.get(&select_resolution);
        let selected = resolution_map.get(&select_resolution);

        let lock_resolution = { self.config.lock().unwrap().lock_resolution };
        if lock_resolution && selected.is_none() {
            return Err(EpisodeDownloadError::ResolutionUnavailable {
                available: resolution_map.keys().map(ToOwned::to_owned).collect(),
            });
        }
        let variant = variant.unwrap_or_else(|| {
            let mut closest_vec = resolution_map
                .iter()
                .map(|(resolution, variant)| (resolution.abs_diff(select_resolution), variant))
                .collect::<Vec<(u64, &VariantStream)>>();
            closest_vec.sort_by_key(|v| v.0);
            let selected = closest_vec[0].1;
            tracing::info!(
                resolution = %selected.resolution.expect("resolution should exist"),
                "could not find selected resolution, chose closest resolution"
            );
            selected
        });

        Ok(variant.to_owned())
    }

    async fn get_media_playlist(
        &self,
        pl_src: &Url,
    ) -> EpisodeDownloadResult<(Bytes, MediaPlaylist)> {
        let playlist_bytes = self
            .request_client
            .get(pl_src, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let (_, media_pl) = parse_media_playlist(&playlist_bytes).finish()?;

        Ok((playlist_bytes, media_pl))
    }
}

#[cfg(test)]
mod m3u8_test {
    use std::error::Error;

    use m3u8_rs::{KeyMethod, Resolution};
    use url::Url;

    #[test]
    fn m3u8_parse_master_playlist() -> Result<(), Box<dyn Error>> {
        let content = br##"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-STREAM-INF:BANDWIDTH=400000,RESOLUTION=640x360
360p/hdntl=exp=1784395675~acl=%2f1141335ecf074e13f1c3e381db61f15d50216b55%2fplaylist_guest.m3u8!%2f1141335ecf074e13f1c3e381db61f15d50216b55%2f360p%2f*~data=hdntl,5279d92ff769460%3a49953%3a0%3a1%3a6868682b%3a0%3a1~hmac=dd2e1220cbd4c8ace80f098acb9469b0eb30f01c7b1bc17d7dbb8e96085716ec/chunklist_b400000.m3u8
"##;
        let (_, pl) = m3u8_rs::parse_master_playlist(content)?;

        println!("{pl:#?}");
        assert_eq!(pl.version, Some(3));
        assert_eq!(pl.variants.len(), 1);
        assert!(!pl.independent_segments);

        let p360 = pl.variants.first().unwrap();
        assert_eq!(
            p360.uri,
            "360p/hdntl=exp=1784395675~acl=%2f1141335ecf074e13f1c3e381db61f15d50216b55%2fplaylist_guest.m3u8!%2f1141335ecf074e13f1c3e381db61f15d50216b55%2f360p%2f*~data=hdntl,5279d92ff769460%3a49953%3a0%3a1%3a6868682b%3a0%3a1~hmac=dd2e1220cbd4c8ace80f098acb9469b0eb30f01c7b1bc17d7dbb8e96085716ec/chunklist_b400000.m3u8"
        );
        assert!(!p360.is_i_frame);
        assert_eq!(p360.bandwidth, 400000);
        assert_eq!(
            p360.resolution,
            Some(Resolution {
                width: 640,
                height: 360
            })
        );

        Ok(())
    }

    #[test]
    fn m3u8_parse_media_playlist() -> Result<(), Box<dyn Error>> {
        let content = br##"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:10
#EXT-X-MEDIA-SEQUENCE:0
#EXT-X-KEY:METHOD=AES-128,URI="key_b400000.m3u8key",IV=0x2b8c048f2fb7a2cefc543a3c1d1c869b
#EXTINF:10.427089,
media_b400000_0.ts
#EXTINF:10.427078,
media_b400000_1.ts
#EXTINF:10.427089,
media_b400000_2.ts
#EXTINF:10.427078,
media_b400000_3.ts
#EXTINF:10.427089,
media_b400000_4.ts
#EXTINF:10.427078,
media_b400000_5.ts
#EXTINF:10.427089,
media_b400000_6.ts
#EXTINF:10.427078,
media_b400000_7.ts
#EXTINF:10.427089,
media_b400000_8.ts
#EXTINF:10.427078,
media_b400000_9.ts
#EXTINF:10.427089,
media_b400000_10.ts
#EXTINF:10.427078,
media_b400000_11.ts
#EXTINF:10.427089,
media_b400000_12.ts
#EXTINF:10.427078,
media_b400000_13.ts
#EXTINF:10.427089,
media_b400000_14.ts
#EXTINF:10.427078,
media_b400000_15.ts
#EXTINF:10.427089,
media_b400000_16.ts
#EXTINF:10.427078,
media_b400000_17.ts
#EXTINF:10.427089,
media_b400000_18.ts
#EXTINF:10.427078,
media_b400000_19.ts
#EXTINF:10.427089,
media_b400000_20.ts
#EXTINF:10.427078,
media_b400000_21.ts
#EXTINF:10.427089,
media_b400000_22.ts
#EXTINF:10.427078,
media_b400000_23.ts
#EXTINF:10.427089,
media_b400000_24.ts
#EXTINF:10.427078,
media_b400000_25.ts
#EXTINF:10.427089,
media_b400000_26.ts
#EXTINF:10.427078,
media_b400000_27.ts
#EXTINF:10.427089,
media_b400000_28.ts
#EXTINF:10.427078,
media_b400000_29.ts
#EXTINF:10.427089,
media_b400000_30.ts
#EXTINF:10.427078,
media_b400000_31.ts
#EXTINF:10.427089,
media_b400000_32.ts
#EXTINF:10.427078,
media_b400000_33.ts
#EXTINF:10.427089,
media_b400000_34.ts
#EXTINF:10.427078,
media_b400000_35.ts
#EXTINF:10.427089,
media_b400000_36.ts
#EXTINF:10.427078,
media_b400000_37.ts
#EXTINF:10.427089,
media_b400000_38.ts
#EXTINF:10.427078,
media_b400000_39.ts
#EXTINF:10.427089,
media_b400000_40.ts
#EXTINF:10.427078,
media_b400000_41.ts
#EXTINF:10.427089,
media_b400000_42.ts
#EXTINF:10.427078,
media_b400000_43.ts
#EXTINF:10.427089,
media_b400000_44.ts
#EXTINF:10.427078,
media_b400000_45.ts
#EXTINF:10.427089,
media_b400000_46.ts
#EXTINF:10.427078,
media_b400000_47.ts
#EXTINF:10.427089,
media_b400000_48.ts
#EXTINF:10.427078,
media_b400000_49.ts
#EXTINF:10.427089,
media_b400000_50.ts
#EXTINF:10.427078,
media_b400000_51.ts
#EXTINF:10.427089,
media_b400000_52.ts
#EXTINF:10.427078,
media_b400000_53.ts
#EXTINF:10.427089,
media_b400000_54.ts
#EXTINF:10.427078,
media_b400000_55.ts
#EXTINF:10.427089,
media_b400000_56.ts
#EXTINF:10.427078,
media_b400000_57.ts
#EXTINF:10.427089,
media_b400000_58.ts
#EXTINF:10.427078,
media_b400000_59.ts
#EXTINF:10.427089,
media_b400000_60.ts
#EXTINF:10.427078,
media_b400000_61.ts
#EXTINF:10.427089,
media_b400000_62.ts
#EXTINF:10.427078,
media_b400000_63.ts
#EXTINF:10.427089,
media_b400000_64.ts
#EXTINF:10.427078,
media_b400000_65.ts
#EXTINF:10.427089,
media_b400000_66.ts
#EXTINF:10.427078,
media_b400000_67.ts
#EXTINF:10.427089,
media_b400000_68.ts
#EXTINF:10.427078,
media_b400000_69.ts
#EXTINF:10.427089,
media_b400000_70.ts
#EXTINF:10.427078,
media_b400000_71.ts
#EXTINF:10.427089,
media_b400000_72.ts
#EXTINF:10.427078,
media_b400000_73.ts
#EXTINF:10.427089,
media_b400000_74.ts
#EXTINF:10.427078,
media_b400000_75.ts
#EXTINF:10.427089,
media_b400000_76.ts
#EXTINF:10.427078,
media_b400000_77.ts
#EXTINF:10.427089,
media_b400000_78.ts
#EXTINF:10.427078,
media_b400000_79.ts
#EXTINF:10.427089,
media_b400000_80.ts
#EXTINF:10.427078,
media_b400000_81.ts
#EXTINF:10.427089,
media_b400000_82.ts
#EXTINF:10.427078,
media_b400000_83.ts
#EXTINF:10.427089,
media_b400000_84.ts
#EXTINF:10.427078,
media_b400000_85.ts
#EXTINF:10.427089,
media_b400000_86.ts
#EXTINF:10.427078,
media_b400000_87.ts
#EXTINF:10.427089,
media_b400000_88.ts
#EXTINF:10.427078,
media_b400000_89.ts
#EXTINF:10.427089,
media_b400000_90.ts
#EXTINF:10.427078,
media_b400000_91.ts
#EXTINF:10.427089,
media_b400000_92.ts
#EXTINF:10.427078,
media_b400000_93.ts
#EXTINF:10.427089,
media_b400000_94.ts
#EXTINF:10.427078,
media_b400000_95.ts
#EXTINF:10.427089,
media_b400000_96.ts
#EXTINF:10.427078,
media_b400000_97.ts
#EXTINF:10.427089,
media_b400000_98.ts
#EXTINF:10.427078,
media_b400000_99.ts
#EXTINF:10.427089,
media_b400000_100.ts
#EXTINF:10.427078,
media_b400000_101.ts
#EXTINF:10.427089,
media_b400000_102.ts
#EXTINF:10.427078,
media_b400000_103.ts
#EXTINF:10.427089,
media_b400000_104.ts
#EXTINF:10.427078,
media_b400000_105.ts
#EXTINF:10.427089,
media_b400000_106.ts
#EXTINF:10.427078,
media_b400000_107.ts
#EXTINF:10.427089,
media_b400000_108.ts
#EXTINF:10.427078,
media_b400000_109.ts
#EXTINF:10.427089,
media_b400000_110.ts
#EXTINF:10.427078,
media_b400000_111.ts
#EXTINF:10.427089,
media_b400000_112.ts
#EXTINF:10.427078,
media_b400000_113.ts
#EXTINF:10.427089,
media_b400000_114.ts
#EXTINF:10.427078,
media_b400000_115.ts
#EXTINF:10.427089,
media_b400000_116.ts
#EXTINF:10.427078,
media_b400000_117.ts
#EXTINF:10.427089,
media_b400000_118.ts
#EXTINF:10.427078,
media_b400000_119.ts
#EXTINF:10.427089,
media_b400000_120.ts
#EXTINF:10.427078,
media_b400000_121.ts
#EXTINF:10.427089,
media_b400000_122.ts
#EXTINF:10.427078,
media_b400000_123.ts
#EXTINF:10.427089,
media_b400000_124.ts
#EXTINF:10.427078,
media_b400000_125.ts
#EXTINF:10.427089,
media_b400000_126.ts
#EXTINF:10.427078,
media_b400000_127.ts
#EXTINF:10.427089,
media_b400000_128.ts
#EXTINF:10.427078,
media_b400000_129.ts
#EXTINF:10.427089,
media_b400000_130.ts
#EXTINF:10.427078,
media_b400000_131.ts
#EXTINF:10.427089,
media_b400000_132.ts
#EXTINF:10.427078,
media_b400000_133.ts
#EXTINF:10.427089,
media_b400000_134.ts
#EXTINF:10.427078,
media_b400000_135.ts
#EXTINF:10.427089,
media_b400000_136.ts
#EXTINF:6.673778,
media_b400000_137.ts
#EXT-X-ENDLIST
"##;

        let (_, pl) = m3u8_rs::parse_media_playlist(content)?;

        println!("{pl:#?}");
        assert_eq!(pl.version, Some(3));
        assert_eq!(pl.target_duration, 10);
        assert_eq!(pl.segments.len(), 138);

        let first_segment = pl.segments.first().unwrap();
        assert_eq!(first_segment.uri, "media_b400000_0.ts");
        assert_eq!(first_segment.duration, 10.427089);
        assert!(first_segment.key.is_some());

        let key = first_segment.key.as_ref().unwrap();
        assert_eq!(key.method, KeyMethod::AES128);
        assert_eq!(key.uri, Some("key_b400000.m3u8key".to_string()));
        assert_eq!(
            key.iv,
            Some("0x2b8c048f2fb7a2cefc543a3c1d1c869b".to_string())
        );

        Ok(())
    }

    #[test]
    fn m3u8_url_path_resolve() -> Result<(), Box<dyn Error>> {
        let url = Url::parse(
            "https://bahamut.akamaized.net/1141335ecf074e13f1c3e381db61f15d50216b55/playlist_guest.m3u8?hdnts=exp%3D1784312875%7Edata%3D5279d92ff769460%3A49953%3A0%3A1%3A6868682b%3A0%3A1%7Eacl%3D%2F1141335ecf074e13f1c3e381db61f15d50216b55%2Fplaylist_guest.m3u8%21%2F1141335ecf074e13f1c3e381db61f15d50216b55%2F360p%2F%2A%7Ehmac%3D3f3176ab6a3e7a64573b85313896d93bd6437ee4f5dceb9f0d2003e166172aa8",
        )?;

        let url =url.join("360p/hdntl=exp=1784395675~acl=%2f1141335ecf074e13f1c3e381db61f15d50216b55%2fplaylist_guest.m3u8!%2f1141335ecf074e13f1c3e381db61f15d50216b55%2f360p%2f*~data=hdntl,5279d92ff769460%3a49953%3a0%3a1%3a6868682b%3a0%3a1~hmac=dd2e1220cbd4c8ace80f098acb9469b0eb30f01c7b1bc17d7dbb8e96085716ec/chunklist_b400000.m3u8")?;

        assert_eq!(
            url.to_string(),
            "https://bahamut.akamaized.net/1141335ecf074e13f1c3e381db61f15d50216b55/360p/hdntl=exp=1784395675~acl=%2f1141335ecf074e13f1c3e381db61f15d50216b55%2fplaylist_guest.m3u8!%2f1141335ecf074e13f1c3e381db61f15d50216b55%2f360p%2f*~data=hdntl,5279d92ff769460%3a49953%3a0%3a1%3a6868682b%3a0%3a1~hmac=dd2e1220cbd4c8ace80f098acb9469b0eb30f01c7b1bc17d7dbb8e96085716ec/chunklist_b400000.m3u8"
        );

        Ok(())
    }
}
