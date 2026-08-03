use ani_dock_core::{Anime, Episode};
use indexmap::IndexMap;

/// same as ani_dock_core::Anime, but pub all fields
pub struct CreateAnime {
    pub sn: u32,
    pub cover: String,
    pub name: String,

    pub series: IndexMap<String, Vec<CreateEpisode>>,
}

/// same ani_dock_core::Episode, but pub all fields
pub struct CreateEpisode {
    pub sn: u32,
    pub cover: String,
    pub episode: u32,
}

impl From<Anime> for CreateAnime {
    fn from(value: Anime) -> Self {
        let (sn, series, cover, name) = value.into_parts();
        Self {
            sn,
            cover,
            name,

            series: series
                .into_iter()
                .map(|(name, episodes)| (name, episodes.into_iter().map(Into::into).collect()))
                .collect(),
        }
    }
}

impl From<Episode> for CreateEpisode {
    fn from(value: Episode) -> Self {
        let (sn, episode, cover) = value.into_parts();

        Self { sn, cover, episode }
    }
}
