use indexmap::IndexMap;

pub struct CreateAnime {
    pub sn: u32,
    pub cover: String,
    pub title: String,

    pub series: IndexMap<String, Vec<CreateEpisode>>,
}

pub struct CreateEpisode {
    pub sn: u32,
    pub cover: String,
    pub episode: u32,
}
