use std::sync::{Arc, Mutex};

use indexmap::IndexMap;

use self::util::get_anime_video_result_from_sn;
use crate::{
    Episode, config::Config, device_id::DeviceId, error::AnimeBuildError, request::RequestClient,
};

pub mod constant;
pub mod episode;
pub mod episode_detail;
pub mod error;
pub mod util;

// TODO listen cookie jar change, save to config file

#[derive(Debug, PartialEq, Eq)]
pub struct Anime {
    /// anime's internal id
    sn: u32,
    episodes: IndexMap<String, Vec<Episode>>,
    /// cover image of this anime
    cover: String,
    /// title of current episode, because we resolve anime using episode's sn
    title: String,
}

impl Anime {
    pub fn sn(&self) -> u32 {
        self.sn
    }

    pub fn episodes(&self) -> &IndexMap<String, Vec<Episode>> {
        &self.episodes
    }

    pub fn cover(&self) -> &str {
        &self.cover
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Anime {
    pub async fn from_episode_sn(
        sn: u32,
        device_id: DeviceId,
        request_client: Arc<RequestClient>,
        config: Arc<Mutex<Config>>,
    ) -> Result<Self, AnimeBuildError> {
        let anime = get_anime_video_result_from_sn(sn, request_client.clone())
            .await?
            .map_err(|err| format!("request anime video info err: {err}"))?;

        let title = anime.anime().title();

        Ok(Self {
            sn: anime.anime().anime_sn().to_owned(),
            episodes: anime
                .anime()
                .episodes()
                .iter()
                .map(|(series_name, episodes)| {
                    (
                        series_name.to_owned(),
                        episodes
                            .iter()
                            .map(|e| {
                                Episode::new(
                                    e.cover(),
                                    e.episode(),
                                    e.video_sn(),
                                    request_client.clone(),
                                    config.clone(),
                                    device_id.clone(),
                                )
                            })
                            .collect::<Vec<Episode>>(),
                    )
                })
                .collect(),
            cover: anime.anime().cover().to_owned(),
            title: title.to_owned(),
        })
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use crate::cookie::Cookie;

    use super::*;

    /// this use real anime's info to test,
    /// it send a real http request to bahamut.
    #[tokio::test]
    async fn get_anime_3499() -> Result<(), Box<dyn Error>> {
        let config = Arc::new(Mutex::new(Config::default()));

        let request_client = Arc::new(RequestClient::new(
            &config.lock().unwrap(),
            &Cookie::default(),
        )?);

        let device_id = DeviceId::default();

        let anime = Anime::from_episode_sn(
            3499,
            device_id.clone(),
            request_client.clone(),
            config.clone(),
        )
        .await?;

        assert_eq!(
            anime,
            Anime {
                sn: 59221,
                cover: String::from("https://p2.bahamut.com.tw/B/ACG/c/21/0000059221.JPG"),
                title: String::from("進擊的巨人 [1]"),
                episodes: IndexMap::from([
                    (
                        String::from("0"),
                        [
                            (1, 3499),
                            (2, 3500),
                            (3, 3514),
                            (4, 3515),
                            (5, 3501),
                            (6, 3502),
                            (7, 3503),
                            (8, 3504),
                            (9, 3505),
                            (10, 3516),
                            (11, 3517),
                            (12, 3506),
                            (13, 3507),
                            (14, 3518),
                            (15, 3508),
                            (16, 3519),
                            (17, 3509),
                            (18, 3510),
                            (19, 3520),
                            (20, 3521),
                            (21, 3511),
                            (22, 3512),
                            (23, 3522),
                            (24, 3523),
                            (25, 3513),
                        ]
                        .into_iter()
                        .map(|(episode, sn)| Episode::new(
                            "",
                            episode,
                            sn,
                            request_client.clone(),
                            config.clone(),
                            device_id.clone(),
                        ))
                        .collect(),
                    ),
                    (
                        String::from("3"),
                        [
                            (1, 20273),
                            (2, 20274),
                            (3, 20275),
                            (4, 20276),
                            (5, 20277),
                            (6, 20278),
                            (7, 20279),
                            (8, 20280),
                            (9, 20281),
                            (10, 20282),
                            (11, 20283),
                            (12, 20284),
                            (13, 20285),
                            (14, 20286),
                            (15, 20287),
                            (16, 20288),
                            (17, 20289),
                            (18, 20290),
                            (19, 20291),
                            (20, 20292),
                            (21, 20293),
                            (22, 20294),
                            (23, 20295),
                            (24, 20296),
                            (25, 20297),
                        ]
                        .into_iter()
                        .map(|(episode, sn)| Episode::new(
                            "",
                            episode,
                            sn,
                            request_client.clone(),
                            config.clone(),
                            device_id.clone()
                        ))
                        .collect(),
                    ),
                ]),
            }
        );

        Ok(())
    }

    /// test of 9
    #[tokio::test]
    async fn get_anime_49780() -> Result<(), Box<dyn Error>> {
        let config = Arc::new(Mutex::new(Config::default()));
        let request_client = Arc::new(RequestClient::new(
            &config.lock().unwrap(),
            &Cookie::default(),
        )?);
        let device_id = DeviceId::default();

        let anime = Anime::from_episode_sn(
            49780,
            device_id.clone(),
            request_client.clone(),
            config.clone(),
        )
        .await?;

        assert_eq!(anime.sn, 114091);
        assert_eq!(
            anime.cover,
            String::from("https://p2.bahamut.com.tw/B/ACG/c/37/0000143537.JPG"),
        );
        assert_eq!(
            anime.title,
            String::from("劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇 [電影]"),
        );
        assert_eq!(
            anime.episodes,
            IndexMap::from([(
                String::from("1"),
                vec![Episode::new(
                    "https://p2.bahamut.com.tw/B/2KU/17/cbef6db0aeab4fafea1631194f1z5qp5.JPG",
                    1,
                    49780,
                    request_client.clone(),
                    config.clone(),
                    device_id.clone(),
                )]
            )])
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_anime_49903() -> Result<(), Box<dyn Error>> {
        let config = Arc::new(Mutex::new(Config::default()));
        let request_client = Arc::new(RequestClient::new(
            &config.lock().unwrap(),
            &Cookie::default(),
        )?);
        let device_id = DeviceId::default();

        let anime = Anime::from_episode_sn(
            49903,
            device_id.clone(),
            request_client.clone(),
            config.clone(),
        )
        .await?;

        assert_eq!(
            anime.title,
            String::from("無職轉生～到了異世界就拿出真本事～第三季 [2]")
        );
        assert_eq!(
            anime.cover,
            String::from("https://p2.bahamut.com.tw/B/ACG/c/64/0000140264.JPG")
        );
        assert_eq!(anime.sn, 114115);

        Ok(())
    }

    /// test of 20273, chinese dubbed version
    #[tokio::test]
    async fn get_anime_20273() -> Result<(), Box<dyn Error>> {
        let config = Arc::new(Mutex::new(Config::default()));
        let request_client = Arc::new(RequestClient::new(
            &config.lock().unwrap(),
            &Cookie::default(),
        )?);
        let device_id = DeviceId::default();

        let anime = Anime::from_episode_sn(
            20273,
            device_id.clone(),
            request_client.clone(),
            config.clone(),
        )
        .await?;

        assert_eq!(anime.sn, 59221);
        assert_eq!(anime.title, String::from("進擊的巨人 [1] [中文配音]"));

        Ok(())
    }
}
