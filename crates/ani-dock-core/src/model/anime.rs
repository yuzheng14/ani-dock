use indexmap::IndexMap;

use crate::Episode;

#[derive(Debug, PartialEq, Eq)]
pub struct Anime {
    /// anime's internal id
    sn: u32,
    series: IndexMap<String, Vec<Episode>>,
    /// cover image of this anime
    cover: String,
    /// anime's name including season
    name: String,
}

impl Anime {
    pub fn new(
        sn: u32,
        series: IndexMap<String, Vec<Episode>>,
        cover: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            sn,
            series,
            cover: cover.into(),
            name: name.into(),
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

    pub fn into_parts(self) -> (u32, IndexMap<String, Vec<Episode>>, String, String) {
        let Self {
            sn,
            series,
            cover,
            name,
        } = self;
        (sn, series, cover, name)
    }
}
