use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    anime: Anime,
    // present in /anime/v1/video.php?videoSn=<sn> interface
    // promote: [],
    // related animes, such as other seasons, films, special episodes, side stories, ...
    // related_anime: [],
    //
    // related_gnn: [],
    // video: VideoAnime,
}

impl Video {
    pub fn anime(&self) -> &Anime {
        &self.anime
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anime {
    /// unknown sn, used in some api
    acg_sn: u32,
    /// sn of anime
    anime_sn: u32,
    /// cover image, aka. anime image
    cover: String,
    director: String,
    /// key is numeric string. main series will usually be `0`.
    episodes: IndexMap<String, Vec<Episode>>,
    /// episode's title of current sn
    title: String,
}

impl Anime {
    pub fn anime_sn(&self) -> u32 {
        self.anime_sn
    }

    pub fn cover(&self) -> &String {
        &self.cover
    }

    pub fn episodes(&self) -> &IndexMap<String, Vec<Episode>> {
        &self.episodes
    }

    pub fn title(&self) -> &String {
        &self.title
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    /// images of current episode, maybe useful(?)
    cover: String,
    /// number of this episode,
    ///
    /// examples: 1, 2, 3, 4, 5, ...
    episode: u32,
    /// sn of this episode
    video_sn: u32,
    /// if this is current episode, then 1, else 0
    state: u32,
}

impl Episode {
    pub fn cover(&self) -> &String {
        &self.cover
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }

    pub fn video_sn(&self) -> u32 {
        self.video_sn
    }
}
