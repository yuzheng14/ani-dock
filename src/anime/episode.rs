use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use indexmap::IndexMap;
use m3u8_rs::{MasterPlaylist, VariantStream, parse_master_playlist};
use nom::Finish;
use tokio::time;
use url::Url;
use wreq::header::REFERER;

use crate::{
    Anime,
    anime::{episode_detail::EpisodeDetail, error::AnimeDownloadError},
    config::Config,
    constant::ORIGIN,
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

#[derive(Debug, Clone)]
pub struct Episode {
    /// images of current episode, maybe usefull(?)
    cover: String,
    /// number of this episode,
    ///
    /// examples: 1, 2, 3, 4, 5, ...
    episode: u32,
    /// sn of this episode
    sn: u32,
    request_client: Arc<RequestClient>,
    // if download concurrently, this should rewrite to RwLock, but current it download one episode
    // at same time
    config: Arc<Mutex<Config>>,
    /// id of current device used by bahumut for identification
    device_id: DeviceId,
}

impl PartialEq for Episode {
    fn eq(&self, other: &Self) -> bool {
        self.cover == other.cover && self.episode == other.episode && self.sn == other.sn
    }
}

impl Eq for Episode {}

impl Episode {
    pub fn new(
        cover: impl Into<String>,
        episode: u32,
        sn: u32,
        request_client: Arc<RequestClient>,
        config: Arc<Mutex<Config>>,
        device_id: DeviceId,
    ) -> Self {
        Self {
            cover: cover.into(),
            episode,
            sn,
            request_client,
            config,
            device_id,
        }
    }

    #[tracing::instrument]
    pub async fn download(&self, anime: Arc<Anime>) -> AnimeDownloadResult {
        if !FFmpeg::exist().await? {
            return Err(FFmpegError::FFmpegNotExist.into());
        }

        self.get_device_id().await?;

        let token = self.gain_access(true).await?;

        self.unlock(None).await?;
        self.check_lock().await?;
        self.unlock(None).await?;
        self.unlock(None).await?;

        if !token.vip() {
            if self.config.lock().unwrap().only_use_vip {
                return Err(AnimeDownloadError::SetOnlyVipButNot);
            }

            tracing::info!(
                self.sn,
                self.episode,
                "start waiting for ads because of not vip account"
            );

            self.start_ad().await?;
            let ads_time = { self.config.lock().unwrap().ads_time as u64 };
            time::sleep(Duration::from_secs(ads_time)).await;
            self.skip_ad().await?;
        } else {
            tracing::info!("recognize vip account, start downloading");
        }

        self.video_start().await?;
        self.check_no_ad().await?;

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

        let episode_detail = EpisodeDetail::from_sn(self.sn, self.request_client.clone()).await?;
        episode_detail.ensure_bangumi_dir().await?;

        let variant = self.get_media_variant(master_pl)?;

        Ok(())
    }
}

type AnimeDownloadResult<T = ()> = Result<T, AnimeDownloadError>;

impl Episode {
    async fn get_device_id(&self) -> AnimeDownloadResult {
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
    async fn gain_access(&self, add_ad_id: bool) -> AnimeDownloadResult<Token> {
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
                        .ok_or(AnimeDownloadError::DeviceIdDidNotExist)?
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

        Ok(token)
    }

    async fn unlock(&self, ttl: Option<u32>) -> AnimeDownloadResult {
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

    async fn check_lock(&self) -> AnimeDownloadResult {
        let url = format!(
            "{ORIGIN}/ajax/checklock.php?device={}&sn={}",
            {
                self.device_id
                    .get_cloned()
                    .ok_or(AnimeDownloadError::DeviceIdDidNotExist)?
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

    async fn start_ad(&self) -> AnimeDownloadResult {
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

    async fn skip_ad(&self) -> AnimeDownloadResult {
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

    async fn video_start(&self) -> AnimeDownloadResult {
        let url = format!("{ORIGIN}/ajax/videoStart.php?sn={}", self.sn);

        self.request_client
            .get(&url, true)
            .header(REFERER, get_referer(self.sn))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn check_no_ad(&self) -> AnimeDownloadResult {
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

        Err(AnimeDownloadError::WaitAdsTimeout)
    }

    async fn get_playlist(&self) -> AnimeDownloadResult<PlaylistSrc> {
        // FIXME it seems like using
        // https://api.gamer.com.tw/anime/v1/video_src.php?videoSn=49953&deviceid=0118ddf4664ceba5c04a12aec5025b4d7c93819911f881586a5a65f00630&deviceTypeUseCases=1
        // now
        let url = format!(
            "{ORIGIN}/ajax/m3u8.php?sn={}&device={}",
            self.sn,
            self.device_id
                .get_cloned()
                .ok_or(AnimeDownloadError::DeviceIdDidNotExist)?
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
            .into_result()?;

        Ok(playlist)
    }

    fn get_media_variant(&self, master_pl: MasterPlaylist) -> AnimeDownloadResult<VariantStream> {
        let resolution_map = master_pl
            .variants
            .into_iter()
            .map(|variant| {
                Ok::<(u64, VariantStream), AnimeDownloadError>((
                    variant
                        .resolution
                        .ok_or(AnimeDownloadError::Plain(
                            "include variant without resolution: \n{variant:#?}".to_string(),
                        ))?
                        .height,
                    variant,
                ))
            })
            .collect::<AnimeDownloadResult<IndexMap<u64, VariantStream>>>()?;

        let select_resolution = { self.config.lock().unwrap().download_resolution.get_height() };
        let variant = resolution_map.get(&select_resolution);
        let selected = resolution_map.get(&select_resolution);

        let lock_resolution = { self.config.lock().unwrap().lock_resolution };
        if lock_resolution && selected.is_none() {
            return Err(AnimeDownloadError::Plain(format!(
                "there is no selected resolution, and locked resolution. available resolutions: {:?}",
                resolution_map
                    .keys()
                    .map(|k| k.to_owned())
                    .collect::<Vec<u64>>()
            )));
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
}
