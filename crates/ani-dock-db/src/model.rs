use chrono::{DateTime, Local};
use indexmap::IndexMap;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Anime {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,

    pub series: IndexMap<String, Vec<Episode>>,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Episode {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub episode: u32,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

impl From<Episode> for ani_dock_core::Episode {
    fn from(value: Episode) -> Self {
        let Episode {
            sn, episode, cover, ..
        } = value;

        Self::new(sn, episode, cover)
    }
}
