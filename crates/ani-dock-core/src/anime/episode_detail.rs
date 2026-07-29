use std::{path::PathBuf, sync::Arc};

use tokio::fs;

use crate::{
    anime::{
        constant::{
            EXTRA_FILTER_REGEX, FULL_EPISODE_REGEX, MIN_EPISODE_REGEX, SEASON_FILTER_REGEX,
            WHITESPACES_REGEX,
        },
        util::get_anime_video_result_from_sn,
    },
    constant::BANGUMI_DIR_PATH,
    request::RequestClient,
    util::sanitize_path_segment,
};

#[derive(Debug, thiserror::Error)]
pub enum EpisodeDetailBuildError {
    #[error("request error when build episode detail: {0}")]
    Request(#[from] wreq::Error),
    #[error("{0}")]
    Plain(String),
}

type EpisodeDetailBuildResult<T = ()> = Result<T, EpisodeDetailBuildError>;

pub struct EpisodeDetail {
    /// exclude `[]`, such as `1` `2` `電影`
    pub(super) episode: String,
    pub(super) season: String,
    pub(super) name: String,
}

impl EpisodeDetail {
    pub async fn from_sn(
        sn: u32,
        request_client: Arc<RequestClient>,
    ) -> EpisodeDetailBuildResult<EpisodeDetail> {
        let video = get_anime_video_result_from_sn(sn, &request_client)
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
        })
    }

    pub async fn ensure_bangumi_dir(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(self.bangumi_dir()).await
    }

    pub fn bangumi_dir(&self) -> PathBuf {
        BANGUMI_DIR_PATH
            .join(sanitize_path_segment(&self.name))
            .join(sanitize_path_segment(&self.season))
    }

    pub fn get_filename(&self, resolution: u64) -> String {
        sanitize_path_segment(&format!(
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
            "Re:Zero/新編集版? [2]",
        ]
    });

    #[test]
    fn get_episode() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            ANIME_TITLES
                .iter()
                .map(|t| EpisodeDetail::get_episode(t).unwrap())
                .collect::<Vec<String>>(),
            vec!["1", "電影", "2", "1", "2"]
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
            vec![None, None, Some(String::from("第三季")), None, None]
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
            vec![None, None, None, Some(String::from("中文配音")), None]
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
            vec!["第一季", "電影", "第三季", "第一季 中文配音", "第一季"]
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
                "進擊的巨人",
                "Re:Zero/新編集版?"
            ]
        );
    }

    #[test]
    fn bangumi_dir() {
        let expected = [
            BANGUMI_DIR_PATH.join("進擊的巨人").join("第一季"),
            BANGUMI_DIR_PATH
                .join("劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇")
                .join("電影"),
            BANGUMI_DIR_PATH
                .join("無職轉生～到了異世界就拿出真本事～")
                .join("第三季"),
            BANGUMI_DIR_PATH.join("進擊的巨人").join("第一季 中文配音"),
            BANGUMI_DIR_PATH.join("Re：Zero／新編集版？").join("第一季"),
        ];

        for (title, expected) in ANIME_TITLES.iter().zip(expected) {
            let episode_detail = EpisodeDetail::from_title(title).unwrap();

            assert_eq!(episode_detail.bangumi_dir(), expected);
        }
    }

    #[test]
    fn get_filename() {
        let expected = [
            (1080, "進擊的巨人[1][1080P].mp4"),
            (
                720,
                "劇場版 關於我轉生變成史萊姆這檔事 蒼海之淚篇[電影][720P].mp4",
            ),
            (1080, "無職轉生～到了異世界就拿出真本事～[2][1080P].mp4"),
            (1080, "進擊的巨人[1][1080P].mp4"),
            (540, "Re：Zero／新編集版？[2][540P].mp4"),
        ];

        for (title, (resolution, expected)) in ANIME_TITLES.iter().zip(expected) {
            let episode_detail = EpisodeDetail::from_title(title).unwrap();

            assert_eq!(episode_detail.get_filename(resolution), expected);
        }
    }
}
