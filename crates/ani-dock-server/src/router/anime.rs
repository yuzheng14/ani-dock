use ani_dock_db::model::Anime;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::{ApiResult, router::AppState};

// pub async fn save_anime(
//     State(state): State<AppState>,
//     Json(anime): Json<Anime>,
// ) -> ApiResult<StatusCode> {
//     state.db.anime.insert(anime.into()).await?;
//
//     Ok(StatusCode::CREATED)
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAnimeRequest {
    sn: u32,
}

pub async fn import_anime(
    State(state): State<AppState>,
    Json(req): Json<ImportAnimeRequest>,
) -> ApiResult<StatusCode> {
    let anime = state.resolver.from_episode_sn(req.sn).await?;

    state.db.anime.insert(anime.into()).await?;

    Ok(StatusCode::CREATED)
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SelectAnimesParam {
    downloaded: Option<bool>,
}

pub async fn select_animes(
    State(state): State<AppState>,
    Query(query): Query<SelectAnimesParam>,
) -> ApiResult<(StatusCode, Json<Vec<Anime>>)> {
    let animes = if let Some(downloaded) = query.downloaded {
        state.db.anime.select_by_download_status(downloaded).await?
    } else {
        state.db.anime.select_all().await?
    };

    Ok((StatusCode::OK, Json(animes)))
}
