use ani_dock_db::model::Anime;
use anyhow::Context;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderName, StatusCode, header::CONTENT_TYPE},
    routing::get,
};
use serde::Deserialize;

use crate::{ApiError, ApiResult, router::AppState, service::request_cover};

// pub async fn save_anime(
//     State(state): State<AppState>,
//     Json(anime): Json<Anime>,
// ) -> ApiResult<StatusCode> {
//     state.db.anime.insert(anime.into()).await?;
//
//     Ok(StatusCode::CREATED)
// }

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(select_animes).post(import_anime))
        .route("/{id_or_sn}/cover", get(get_cover))
}

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

pub async fn get_cover(
    State(state): State<AppState>,
    Path(id_or_sn): Path<String>,
) -> ApiResult<([(HeaderName, String); 1], Bytes)> {
    let anime = state
        .db
        .anime
        .select_row_by_id_or_sn(&id_or_sn)
        .await
        .context("查询动画数据出错")?
        .ok_or(ApiError::NotFound)?;

    let (mime_type, bytes) = if let Some(cover_id) = anime.cover_id {
        let cover_image = state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .context("查询封面数据库出错")?;
        (cover_image.mime_type, cover_image.bytes)
    } else {
        let episode = state
            .db
            .episode
            .select_one_by_anime_id(anime.id)
            .await?
            .ok_or(ApiError::NotFound)?;

        let (mime_type, bytes, cover_id) = request_cover(
            &state.request_client,
            &state.db.cover_image,
            &anime.cover,
            episode.sn,
        )
        .await?;

        state
            .db
            .anime
            .update_cover_id(anime.id, cover_id)
            .await
            .context("更新剧集的封面资源引用出错")?;

        (mime_type, bytes)
    };

    Ok(([(CONTENT_TYPE, mime_type)], bytes))
}
