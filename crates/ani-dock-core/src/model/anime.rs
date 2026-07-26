use indexmap::IndexMap;

use crate::Episode;

#[derive(Debug, PartialEq, Eq)]
pub struct Anime {
    /// anime's internal id
    sn: u32,
    series: IndexMap<String, Vec<Episode>>,
    /// cover image of this anime
    cover: String,
}

impl Anime {
    pub fn new(sn: u32, series: IndexMap<String, Vec<Episode>>, cover: impl Into<String>) -> Self {
        Self {
            sn,
            series,
            cover: cover.into(),
        }
    }

    pub fn sn(&self) -> u32 {
        self.sn
    }

    pub fn series(&self) -> &IndexMap<String, Vec<Episode>> {
        &self.series
    }

    pub fn cover(&self) -> &str {
        &self.cover
    }

    pub fn into_parts(self) -> (u32, IndexMap<String, Vec<Episode>>, String) {
        let Self { sn, series, cover } = self;
        (sn, series, cover)
    }
}
