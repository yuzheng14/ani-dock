use ani_dock_core::Anime;
use indexmap::IndexMap;

pub struct CreateAnime {
    pub sn: u32,
    pub cover: String,

    pub series: IndexMap<String, Vec<CreateEpisode>>,
}

pub struct CreateEpisode {
    pub sn: u32,
    pub cover: String,
    pub episode: u32,
}

impl From<Anime> for CreateAnime {
    fn from(value: Anime) -> Self {
        let (sn, series, cover) = value.into_parts();
        Self {
            sn,
            cover,

            series: series
                .into_iter()
                .map(|(name, episodes)| {
                    (
                        name,
                        episodes
                            .into_iter()
                            .map(|e| {
                                let (sn, episode, cover) = e.into_parts();
                                CreateEpisode { sn, cover, episode }
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}
