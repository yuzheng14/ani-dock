use std::sync::Arc;

use tokio::fs;

use crate::{
    anime::{
        constant::{
            EXTRA_FILTER_REGEX, FULL_EPISODE_REGEX, MIN_EPISODE_REGEX, SEASON_FILTER_REGEX,
            WHITESPACES_REGEX,
        },
        error::EpisodeDetailBuildError,
        util::get_anime_video_result_from_sn,
    },
    constant::BANGUMI_DIR_PATH,
    request::RequestClient,
    util::santitize_path_segment,
};

type EpisodeDetailBuildResult<T = ()> = Result<T, EpisodeDetailBuildError>;

pub struct EpisodeDetail {
    /// exclude `[]`, such as `1` `2` `電影`
    pub(super) episode: String,
    pub(super) season: String,
    pub(super) name: String,
    pub(super) title: String,
}

impl EpisodeDetail {
    pub async fn from_sn(
        sn: u32,
        request_client: Arc<RequestClient>,
    ) -> EpisodeDetailBuildResult<EpisodeDetail> {
        let video = get_anime_video_result_from_sn(sn, request_client)
            .await?
            .map_err(|err| {
                EpisodeDetailBuildError::Plain(format!("request episode sn={sn} info error: {err}"))
            })?;

        let title = video.anime().title();

        Self::from_title(title)
    }

    fn from_title(title: &str) -> EpisodeDetailBuildResult<Self> {
        let episode = Self::get_episode(title)?;
        let season = Self::get_season(title);
        let extra = Self::get_extra(title);
        let season_with_extra =
            Self::get_season_with_extra(&episode, season.as_deref(), extra.as_deref());

        let name = Self::get_name(title, season.as_deref(), extra.as_deref(), &episode);

        Ok(Self {
            episode,
            season: season_with_extra,
            name,
            title: title.to_owned(),
        })
    }

    pub async fn ensure_bangumi_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(
            BANGUMI_DIR_PATH
                .join(santitize_path_segment(&self.name))
                .join(santitize_path_segment(&self.season)),
        )
        .await
    }

    pub fn get_filename(&self, resolution: u64) -> String {
        santitize_path_segment(&format!(
            "{}[{}][{}P].mp4",
            self.name, self.episode, resolution
        ))
    }
}

impl EpisodeDetail {
    fn get_episode(title: &str) -> EpisodeDetailBuildResult<String> {
        MIN_EPISODE_REGEX
            .find(title)
            .map(|find| find.as_str())
            .or_else(|| {
                FULL_EPISODE_REGEX
                    .find_iter(title)
                    .last()
                    .map(|find| find.as_str())
            })
            .map(|ep| ep[1..ep.len() - 1].to_owned())
            .ok_or(EpisodeDetailBuildError::Plain(String::from(
                "could not find episode",
            )))
    }

    fn get_season(title: &str) -> Option<String> {
        SEASON_FILTER_REGEX
            .find(title)
            .map(|find| find.as_str().to_owned())
    }

    fn get_extra(title: &str) -> Option<String> {
        EXTRA_FILTER_REGEX
            .find(title)
            .map(|find| find.as_str()[1..find.len() - 1].to_owned())
    }

    fn get_season_with_extra(episode: &str, season: Option<&str>, extra: Option<&str>) -> String {
        match (season, extra) {
            // season 2 with dubbed will be `第二季 中文配音`
            (Some(season), Some(extra)) => format!("{season} {extra}"),
            (Some(season), None) => season.to_owned(),
            (None, Some(extra)) => format!("第一季 {extra}"),
            (None, None) => {
                let movie_string = String::from("電影");

                if episode == movie_string {
                    movie_string
                } else {
                    String::from("第一季")
                }
            }
        }
    }

    fn get_name(title: &str, season: Option<&str>, extra: Option<&str>, episode: &str) -> String {
        let mut plain_title = title.replace(&format!("[{episode}]"), "");
        if let Some(season) = season.as_ref() {
            plain_title = plain_title.replace(season, "");
        }
        if let Some(extra) = extra {
            plain_title = plain_title.replace(&format!("[{extra}]"), "");
        }
        WHITESPACES_REGEX
            .replace_all(plain_title.trim(), " ")
            .to_string()
    }
}

#[cfg(test)]
mod test {
    use std::{error::Error, sync::LazyLock};

    use super::*;

    static ANIME_TITLES: LazyLock<Vec<&str>> = LazyLock::new(|| {
        vec![
            "進擊的巨人 [1]",
            "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇 [電影]",
            "無職轉生～到了異世界就拿出真本事～第三季 [2]",
            "進擊的巨人 [1] [中文配音]",
        ]
    });

    #[test]
    fn get_episode() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_episode(t).unwrap())
                .collect::<Vec<String>>(),
            vec!["1", "電影", "2", "1"]
        );

        Ok(())
    }

    #[test]
    fn get_season() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_season(t))
                .collect::<Vec<Option<String>>>(),
            vec![None, None, Some(String::from("第三季")), None]
        );

        Ok(())
    }

    #[test]
    fn get_extra() {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_extra(t))
                .collect::<Vec<Option<String>>>(),
            vec![None, None, None, Some(String::from("中文配音"))]
        )
    }

    #[test]
    fn get_season_with_extra() {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_season_with_extra(
                    &EpisodeDetail::get_episode(t).unwrap(),
                    EpisodeDetail::get_season(t).as_deref(),
                    EpisodeDetail::get_extra(t).as_deref()
                ))
                .collect::<Vec<String>>(),
            vec!["第一季", "電影", "第三季", "第一季 中文配音"]
        )
    }

    #[test]
    fn get_name() {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_name(
                    t,
                    EpisodeDetail::get_season(t).as_deref(),
                    EpisodeDetail::get_extra(t).as_deref(),
                    &EpisodeDetail::get_episode(t).unwrap()
                ))
                .collect::<Vec<String>>(),
            vec![
                "進擊的巨人",
                "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇",
                "無職轉生～到了異世界就拿出真本事～",
                "進擊的巨人"
            ]
        );
    }

    #[test]
    fn get_filename() {
        let cases = [
            ("進擊的巨人 [1]", 1080, "進擊的巨人[1][1080P].mp4"),
            (
                "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇 [電影]",
                720,
                "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇[電影][720P].mp4",
            ),
            (
                "Re:Zero/新編集版? [2]",
                540,
                "Re：Zero／新編集版？[2][540P].mp4",
            ),
        ];

        for (title, resolution, expected) in cases {
            let episode_detail = EpisodeDetail::from_title(title).unwrap();

            assert_eq!(episode_detail.get_filename(resolution), expected);
        }
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
