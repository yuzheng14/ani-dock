// TODO include details directly
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    /// sn of this episode
    sn: u32,
    /// number of this episode,
    ///
    /// examples: 1, 2, 3, 4, 5, ...
    episode: u32,
    /// images of current episode, maybe usefull(?)
    cover: String,
}

impl Episode {
    pub fn new(sn: u32, episode: u32, cover: impl Into<String>) -> Self {
        Self {
            cover: cover.into(),
            episode,
            sn,
        }
    }

    pub fn cover(&self) -> &str {
        &self.cover
    }

    pub fn episode(&self) -> u32 {
        self.episode
    }

    pub fn sn(&self) -> u32 {
        self.sn
    }

    pub fn into_parts(self) -> (u32, u32, String) {
        let Self { sn, episode, cover } = self;

        (sn, episode, cover)
    }
}
