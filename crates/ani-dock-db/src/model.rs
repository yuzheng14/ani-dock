use chrono::{DateTime, Local};
use indexmap::IndexMap;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Anime {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,

    pub series: IndexMap<String, Vec<Episode>>,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

#[derive(Debug, Serialize)]
pub struct Episode {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub episode: u32,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}
