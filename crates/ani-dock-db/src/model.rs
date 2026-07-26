use chrono::{DateTime, Local};
use indexmap::IndexMap;
use uuid::Uuid;

pub struct Anime {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,

    pub series: IndexMap<String, Vec<Episode>>,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

pub struct Episode {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub episode: u32,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}
