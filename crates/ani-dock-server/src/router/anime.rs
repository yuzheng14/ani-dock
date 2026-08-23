use ani_dock_db::model::Anime;
use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::get,
};
use serde::Deserialize;

use crate::{
    ApiResult,
    router::{AppState, cover},
    service::request_cover,
};

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
    request_headers: HeaderMap,
) -> ApiResult<Response> {
    let Some(anime) = state
        .db
        .anime
        .select_row_by_id_or_sn(&id_or_sn)
        .await
        .context("查询动画数据出错")?
    else {
        return Ok(cover::not_found());
    };

    let cover_image = if let Some(cover_id) = anime.cover_id {
        state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .context("查询封面数据库出错")?
    } else {
        let Some(episode) = state
            .db
            .episode
            .select_one_by_anime_id(anime.id)
            .await?
        else {
            return Ok(cover::not_found());
        };

        let cover_image = request_cover(
            &state.request_client,
            &state.db.cover_image,
            &anime.cover,
            episode.sn,
        )
        .await?;

        state
            .db
            .anime
            .update_cover_id(anime.id, cover_image.id)
            .await
            .context("更新剧集的封面资源引用出错")?;

        cover_image
    };

    Ok(cover::response(&request_headers, cover_image))
}

#[cfg(test)]
mod tests {
    use ani_dock_db::{
        input::{CreateAnime, CreateEpisode},
        repository::AnimeRepository,
    };
    use axum::{
        body::to_bytes,
        extract::{Path, State},
        http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    };
    use indexmap::IndexMap;
    use sqlx::SqlitePool;

    use super::*;
    use crate::router::test_helpers::{app_state, image_server};

    const IMAGE_BYTES: &[u8] = b"test anime cover";

    async fn insert_anime(pool: &SqlitePool, cover: &str) -> Anime {
        let repository = AnimeRepository::new(pool.clone());
        repository
            .insert(CreateAnime {
                sn: 59_221,
                cover: cover.to_owned(),
                name: "進擊的巨人".to_owned(),
                series: IndexMap::from([(
                    "本篇".to_owned(),
                    vec![CreateEpisode {
                        sn: 3_499,
                        cover: String::new(),
                        episode: 1,
                    }],
                )]),
            })
            .await
            .expect("anime fixture should be inserted");

        repository
            .select_row_by_id_or_sn("59221")
            .await
            .expect("anime fixture query should succeed")
            .expect("anime fixture should exist")
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_fetches_persists_and_reuses_anime_image(pool: SqlitePool) {
        let image_server = image_server(IMAGE_BYTES, "image/webp").await;
        let anime = insert_anime(&pool, image_server.url()).await;
        assert_eq!(anime.cover_id, None);
        let state = app_state(pool);

        let response = get_cover(
            State(state.clone()),
            Path(anime.id.to_string()),
            HeaderMap::new(),
        )
        .await
        .expect("first cover request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/webp");
        assert_eq!(response.headers()[CACHE_CONTROL], cover::CACHE_CONTROL_VALUE);
        let etag = response.headers()[ETAG].clone();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("cover response body should be readable");
        assert_eq!(bytes.as_ref(), IMAGE_BYTES);

        let stored_anime = state
            .db
            .anime
            .select_row_by_id_or_sn(&anime.id.to_string())
            .await
            .expect("stored anime query should succeed")
            .expect("stored anime should exist");
        let cover_id = stored_anime
            .cover_id
            .expect("first request should link the stored cover");
        let stored_cover = state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .expect("stored cover should exist");
        assert_eq!(stored_cover.url, image_server.url());
        assert_eq!(stored_cover.mime_type, "image/webp");
        assert_eq!(stored_cover.bytes.as_ref(), IMAGE_BYTES);
        let expected_etag = format!("\"{cover_id}\"");
        assert_eq!(
            etag.to_str().expect("ETag should contain valid text"),
            expected_etag.as_str()
        );
        assert_eq!(image_server.request_count(), 1);
        drop(image_server);

        let cached_response = get_cover(
            State(state.clone()),
            Path(anime.sn.to_string()),
            HeaderMap::new(),
        )
        .await
        .expect("cached cover request should succeed by anime sn");
        assert_eq!(cached_response.status(), StatusCode::OK);
        assert_eq!(cached_response.headers()[ETAG], etag);
        let cached_bytes = to_bytes(cached_response.into_body(), usize::MAX)
            .await
            .expect("cached cover response body should be readable");
        assert_eq!(cached_bytes.as_ref(), IMAGE_BYTES);

        let mut request_headers = HeaderMap::new();
        request_headers.insert(IF_NONE_MATCH, etag);
        let not_modified = get_cover(
            State(state),
            Path(anime.id.to_string()),
            request_headers,
        )
        .await
        .expect("matching ETag should succeed");
        assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            not_modified.headers()[ETAG]
                .to_str()
                .expect("ETag should contain valid text"),
            expected_etag.as_str()
        );
        assert_eq!(
            not_modified.headers()[CACHE_CONTROL],
            cover::CACHE_CONTROL_VALUE
        );
        assert!(
            to_bytes(not_modified.into_body(), usize::MAX)
                .await
                .expect("not modified response body should be readable")
                .is_empty()
        );
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_returns_not_found_for_unknown_anime(pool: SqlitePool) {
        let response = get_cover(
            State(app_state(pool)),
            Path("99999".to_owned()),
            HeaderMap::new(),
        )
        .await
        .expect("unknown anime should produce an HTTP response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers()[CACHE_CONTROL],
            cover::NOT_FOUND_CACHE_CONTROL_VALUE
        );
    }
}
