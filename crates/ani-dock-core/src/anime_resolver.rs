use std::sync::Arc;

use thiserror::Error;

use crate::{Anime, Episode, RequestClient, anime::util::get_anime_video_result_from_sn};

#[derive(Debug, Error)]
pub enum AnimeResolveError {
    #[error("动画疯接口请求错误：{0}")]
    Api(String),

    #[error("HTTP 请求失败: {0}")]
    Http(#[from] wreq::Error),
}

pub type AnimeResolveResult<T = ()> = Result<T, AnimeResolveError>;

#[derive(Debug)]
pub struct AnimeResolver {
    request_client: Arc<RequestClient>,
}

impl AnimeResolver {
    pub fn new(request_client: Arc<RequestClient>) -> Self {
        Self { request_client }
    }

    pub async fn from_episode_sn(&self, sn: u32) -> AnimeResolveResult<Anime> {
        let anime = get_anime_video_result_from_sn(sn, &self.request_client)
            .await?
            .map_err(|err| AnimeResolveError::Api(err.to_string()))?
            .anime()
            .to_owned();

        let sn = anime.anime_sn();
        let cover = anime.cover();
        let series = anime
            .episodes()
            .iter()
            .map(|(name, episodes)| {
                (
                    name.to_owned(),
                    episodes
                        .iter()
                        .map(|e| Episode::new(e.video_sn(), e.episode(), e.cover()))
                        .collect(),
                )
            })
            .collect();

        Ok(Anime::new(sn, series, cover))
    }
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use indexmap::IndexMap;

    use crate::{Config, TestResult, cookie::Cookie};

    use super::*;

    fn get_resolver() -> TestResult<AnimeResolver> {
        let config = Arc::new(Mutex::new(Config::default()));

        let request_client = Arc::new(RequestClient::new(
            &config.lock().unwrap(),
            &Cookie::default(),
        )?);

        Ok(AnimeResolver::new(request_client.clone()))
    }

    /// this use real anime's info to test,
    /// it send a real http request to bahamut.
    #[tokio::test]
    async fn get_anime_3499() -> TestResult {
        let resolver = get_resolver()?;
        let anime = resolver.from_episode_sn(3499).await?;

        assert_eq!(anime.sn(), 59221);
        assert_eq!(
            anime.cover(),
            "https://p2.bahamut.com.tw/B/ACG/c/21/0000059221.JPG"
        );
        assert_eq!(
            anime.series(),
            &IndexMap::from([
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
                    .map(|(episode, sn)| Episode::new(sn, episode, ""))
                    .collect::<Vec<Episode>>(),
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
                    .map(|(episode, sn)| Episode::new(sn, episode, ""))
                    .collect::<Vec<Episode>>(),
                ),
            ]),
        );

        Ok(())
    }

    /// test of 9
    #[tokio::test]
    async fn get_anime_49780() -> TestResult {
        let resolver = get_resolver()?;

        let anime = resolver.from_episode_sn(49780).await?;

        assert_eq!(anime.sn(), 114091);
        assert_eq!(
            anime.cover(),
            "https://p2.bahamut.com.tw/B/ACG/c/37/0000143537.JPG",
        );
        assert_eq!(
            anime.series(),
            &IndexMap::from([(
                String::from("1"),
                vec![Episode::new(
                    49780,
                    1,
                    "https://p2.bahamut.com.tw/B/2KU/17/cbef6db0aeab4fafea1631194f1z5qp5.JPG",
                )]
            )])
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_anime_49903() -> TestResult {
        let resolver = get_resolver()?;

        let anime = resolver.from_episode_sn(49903).await?;

        assert_eq!(
            anime.cover(),
            "https://p2.bahamut.com.tw/B/ACG/c/64/0000140264.JPG"
        );
        assert_eq!(anime.sn(), 114115);

        Ok(())
    }

    /// test of 20273, chinese dubbed version
    #[tokio::test]
    async fn get_anime_20273() -> TestResult {
        let resolver = get_resolver()?;

        let anime = resolver.from_episode_sn(20273).await?;

        assert_eq!(anime.sn(), 59221);

        Ok(())
    }
}
