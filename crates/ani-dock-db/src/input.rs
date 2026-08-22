use bytes::Bytes;
use chrono::{DateTime, Local};
use indexmap::IndexMap;
use sqlx::types::Json;
use uuid::{Uuid, fmt::Hyphenated};

use crate::{
    CoreAnime, CoreEpisode,
    model::{Anime, Episode},
};

/// same as ani_dock_core::Anime, but pub all fields
pub struct CreateAnime {
    pub sn: u32,
    pub cover: String,
    pub name: String,

    pub series: IndexMap<String, Vec<CreateEpisode>>,
}

pub struct AnimeRow {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub cover_id: Option<Hyphenated>,
    pub name: String,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

impl From<AnimeRow> for Anime {
    fn from(value: AnimeRow) -> Self {
        Self {
            id: value.id,
            sn: value.sn,
            cover: value.cover,
            cover_id: value.cover_id.map(|v| v.into_uuid()),
            name: value.name,
            series: IndexMap::new(),
            create_at: value.create_at,
            update_at: value.update_at,
        }
    }
}

pub struct AnimeRowWithSeries {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub cover_id: Option<Hyphenated>,
    pub name: String,

    pub series: Json<IndexMap<String, Vec<Episode>>>,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

pub struct EpisodeRow {
    pub id: Uuid,
    pub sn: u32,
    pub cover: String,
    pub cover_id: Option<Hyphenated>,
    pub episode: u32,

    pub series_id: Uuid,

    pub create_at: DateTime<Local>,
    pub update_at: DateTime<Local>,
}

impl From<EpisodeRow> for Episode {
    fn from(value: EpisodeRow) -> Self {
        Self {
            id: value.id,
            sn: value.sn,
            cover: value.cover,
            cover_id: value.cover_id.map(|v| v.into_uuid()),
            episode: value.episode,
            create_at: value.create_at,
            update_at: value.update_at,
        }
    }
}

impl From<AnimeRowWithSeries> for Anime {
    fn from(value: AnimeRowWithSeries) -> Self {
        Self {
            id: value.id,
            sn: value.sn,
            cover: value.cover,
            cover_id: value.cover_id.map(|v| v.into_uuid()),
            name: value.name,
            series: value.series.0,
            create_at: value.create_at,
            update_at: value.update_at,
        }
    }
}

/// same ani_dock_core::Episode, but pub all fields
pub struct CreateEpisode {
    pub sn: u32,
    pub cover: String,
    pub episode: u32,
}

impl From<CoreAnime> for CreateAnime {
    fn from(value: CoreAnime) -> Self {
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

impl From<CoreEpisode> for CreateEpisode {
    fn from(value: CoreEpisode) -> Self {
        let (sn, episode, cover) = value.into_parts();

        Self { sn, cover, episode }
    }
}

pub struct CreateCoverImage {
    pub url: String,
    pub bytes: Bytes,
    pub mime_type: String,
}
