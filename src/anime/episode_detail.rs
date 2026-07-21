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
};

type EpisodeDetailBuildResult<T = ()> = Result<T, EpisodeDetailBuildError>;

pub struct EpisodeDetail {
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
        fs::create_dir_all(BANGUMI_DIR_PATH.join(&self.name).join(&self.season)).await
    }

    pub fn get_filename(&self) -> String {
        todo!()
    }
}

impl EpisodeDetail {
    // TODO move test related to this file, to remove `pub(super)`
    pub(super) fn get_episode(title: &str) -> EpisodeDetailBuildResult<String> {
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

    pub(super) fn get_season(title: &str) -> Option<String> {
        SEASON_FILTER_REGEX
            .find(title)
            .map(|find| find.as_str().to_owned())
    }

    pub(super) fn get_extra(title: &str) -> Option<String> {
        EXTRA_FILTER_REGEX
            .find(title)
            .map(|find| find.as_str()[1..find.len() - 1].to_owned())
    }

    pub(super) fn get_season_with_extra(
        episode: &str,
        season: Option<&str>,
        extra: Option<&str>,
    ) -> String {
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

    pub(super) fn get_name(
        title: &str,
        season: Option<&str>,
        extra: Option<&str>,
        episode: &str,
    ) -> String {
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
