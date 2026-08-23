use std::convert::Infallible;

use ani_dock_db::{
    model::Episode,
    repository::{DbResult, EpisodeRepository},
};
use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, put},
};
use futures::{Stream, StreamExt, future::try_join_all, stream};
use serde::Serialize;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use ts_rs::TS;

use crate::{
    ApiError, ApiResult, CoreEpisode,
    router::{AppState, cover},
    service::{DownloadState, DownloadStatus, request_cover},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/download", put(download))
        .route("/undownloaded", get(get_undownload_episodes))
        .route("/download/events", get(download_events))
        .route("/{id_or_sn}/cover", get(get_cover))
}

pub async fn download(
    State(state): State<AppState>,
    Json(sn_list): Json<Vec<u32>>,
) -> ApiResult<StatusCode> {
    let episodes = try_join_all(sn_list.into_iter().map(|sn| {
        // let compiler happy
        let state = state.clone();
        async move { Ok::<_, sqlx::Error>((sn, state.db.episode.select(sn).await?)) }
    }))
    .await?;

    let episodes = episodes
        .into_iter()
        .map(|(sn, e)| e.ok_or(ApiError::EpisodeNotFound(sn)).map(Into::into))
        .collect::<Result<Vec<CoreEpisode>, ApiError>>()?;

    for episode in episodes {
        if state
            .db
            .download_queue
            .insert_by_episode_or_ignore(episode.clone())
            .await?
            || state.services.download.is_error(episode.sn())
        {
            state.services.download.schedule_download(episode.clone());
        }
    }

    Ok(StatusCode::ACCEPTED)
}

pub async fn get_undownload_episodes(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Episode>>> {
    let episodes = try_join_all(
        state
            .services
            .download
            .get_undownloaded_episodes_sn()
            .into_iter()
            .map(async |sn| state.db.episode.select(sn).await),
    )
    .await?
    .into_iter()
    .flatten()
    .collect();

    Ok(Json(episodes))
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DownloadEvent {
    episode: Episode,
    state: DownloadState,
}

impl DownloadEvent {
    pub async fn from_download_status(
        ds: DownloadStatus,
        repo: &EpisodeRepository,
    ) -> DbResult<Self> {
        let episode = repo.select(ds.sn).await?.ok_or(sqlx::Error::RowNotFound)?;

        Ok(Self {
            episode,
            state: ds.state,
        })
    }
}

pub async fn download_events(
    State(state): State<AppState>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let shutdown = state.shutdown.clone();
    let rx = state.services.download.subscribe();

    let snapshot = state.services.download.state_snapshot();

    let snapshot = try_join_all(
        snapshot
            .into_iter()
            .map(async |ds| DownloadEvent::from_download_status(ds, &state.db.episode).await),
    )
    .await?;

    let snapshot = Event::default()
        .event("snapshot")
        .json_data(snapshot)
        .map_err(ApiError::SSEEventJsonDataConvert)?;

    let repo = state.db.episode.clone();
    let updates = BroadcastStream::new(rx).filter_map(move |ds| {
        let repo = repo.clone();
        async move {
            let de = match ds {
                Ok(ds) => match DownloadEvent::from_download_status(ds, &repo).await {
                    Ok(de) => de,
                    Err(err) => {
                        tracing::error!(error = %err, "下载接收端解析剧集失败");
                        return None;
                    }
                },
                Err(BroadcastStreamRecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "下载接收端发生滞后");
                    return None;
                }
            };

            let event = match Event::default().event("update").json_data(de) {
                Ok(event) => event,
                Err(err) => {
                    let err = ApiError::SSEEventJsonDataConvert(err);
                    tracing::error!(error = %err);
                    return None;
                }
            };

            Some(Ok::<_, Infallible>(event))
        }
    });

    let stream = stream::once(async { Ok::<_, Infallible>(snapshot) })
        .chain(updates)
        .take_until(shutdown.cancelled_owned());

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn get_cover(
    State(state): State<AppState>,
    Path(id_or_sn): Path<String>,
    request_headers: HeaderMap,
) -> ApiResult<Response> {
    let Some(episode) = state
        .db
        .episode
        .select_one_by_id_or_sn(&id_or_sn)
        .await
        .context("查询剧集信息出错")?
    else {
        return Ok(cover::not_found());
    };

    let cover_image = if let Some(cover_id) = episode.cover_id {
        state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .context("查询剧集信息出错")?
    } else if episode.cover.is_empty() {
        return Ok(cover::not_found());
    } else {
        let cover_image = request_cover(
            &state.request_client,
            &state.db.cover_image,
            &episode.cover,
            episode.sn,
        )
        .await?;

        state
            .db
            .episode
            .update_cover_id(episode.id, cover_image.id)
            .await
            .context("更新剧集的封面资源引用出错")?;

        cover_image
    };

    Ok(cover::response(&request_headers, cover_image))
}

#[cfg(test)]
mod download_events_tests {
    use std::time::Duration;

    use axum::{body::to_bytes, response::IntoResponse};
    use sqlx::SqlitePool;
    use tokio::time;

    use super::*;
    use crate::router::test_helpers::app_state;

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn download_events_stream_ends_on_shutdown(pool: SqlitePool) {
        let state = app_state(pool);
        let shutdown = state.shutdown.clone();
        let response = download_events(State(state))
            .await
            .expect("download event stream should be created")
            .into_response();
        let body_task =
            tokio::spawn(async move { to_bytes(response.into_body(), usize::MAX).await });

        tokio::task::yield_now().await;
        shutdown.cancel();

        time::timeout(Duration::from_secs(1), body_task)
            .await
            .expect("SSE body should end after shutdown")
            .expect("SSE body task should not panic")
            .expect("SSE body should be readable");
    }
}

#[cfg(test)]
mod cover_tests {
    use ani_dock_db::{
        input::{CreateAnime, CreateEpisode},
        repository::{AnimeRepository, EpisodeRepository},
    };
    use axum::{
        body::to_bytes,
        extract::{Path, State},
        http::{
            HeaderValue,
            header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
        },
    };
    use indexmap::IndexMap;
    use sqlx::SqlitePool;

    use super::*;
    use crate::router::test_helpers::{app_state, image_server};

    const IMAGE_BYTES: &[u8] = b"test episode cover";

    async fn insert_episode(pool: &SqlitePool, cover: &str) -> Episode {
        let anime_repository = AnimeRepository::new(pool.clone());
        anime_repository
            .insert(CreateAnime {
                sn: 59_221,
                cover: "https://example.com/anime.jpg".to_owned(),
                name: "進擊的巨人".to_owned(),
                series: IndexMap::from([(
                    "本篇".to_owned(),
                    vec![CreateEpisode {
                        sn: 3_499,
                        cover: cover.to_owned(),
                        episode: 1,
                    }],
                )]),
            })
            .await
            .expect("episode fixture should be inserted");

        EpisodeRepository::new(pool.clone())
            .select(3_499)
            .await
            .expect("episode fixture query should succeed")
            .expect("episode fixture should exist")
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_fetches_persists_and_reuses_episode_image(pool: SqlitePool) {
        let image_server = image_server(IMAGE_BYTES, "image/png").await;
        let episode = insert_episode(&pool, image_server.url()).await;
        assert_eq!(episode.cover_id, None);
        let state = app_state(pool);

        let response = get_cover(
            State(state.clone()),
            Path(episode.id.to_string()),
            HeaderMap::new(),
        )
        .await
        .expect("first cover request should succeed");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
        assert_eq!(
            response.headers()[CACHE_CONTROL],
            cover::CACHE_CONTROL_VALUE
        );
        let etag = response.headers()[ETAG].clone();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("cover response body should be readable");
        assert_eq!(bytes.as_ref(), IMAGE_BYTES);

        let stored_episode = state
            .db
            .episode
            .select(episode.sn)
            .await
            .expect("stored episode query should succeed")
            .expect("stored episode should exist");
        let cover_id = stored_episode
            .cover_id
            .expect("first request should link the stored cover");
        let stored_cover = state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .expect("stored cover should exist");
        assert_eq!(stored_cover.url, image_server.url());
        assert_eq!(stored_cover.mime_type, "image/png");
        assert_eq!(stored_cover.bytes.as_ref(), IMAGE_BYTES);
        let expected_etag = format!("\"{cover_id}\"");
        assert_eq!(
            etag.to_str().expect("ETag should contain valid text"),
            expected_etag.as_str()
        );
        assert_eq!(image_server.request_count(), 1);
        drop(image_server);

        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_static("\"different-cover\""),
        );
        let cached_response =
            get_cover(State(state), Path(episode.sn.to_string()), request_headers)
                .await
                .expect("non-matching ETag should return the cached cover");
        assert_eq!(cached_response.status(), StatusCode::OK);
        assert_eq!(cached_response.headers()[ETAG], etag);
        let cached_bytes = to_bytes(cached_response.into_body(), usize::MAX)
            .await
            .expect("cached cover response body should be readable");
        assert_eq!(cached_bytes.as_ref(), IMAGE_BYTES);
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_returns_not_found_when_episode_cover_is_empty(pool: SqlitePool) {
        let episode = insert_episode(&pool, "").await;
        let state = app_state(pool);

        let response = get_cover(State(state), Path(episode.id.to_string()), HeaderMap::new())
            .await
            .expect("empty episode cover should produce an HTTP response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers()[CACHE_CONTROL],
            cover::NOT_FOUND_CACHE_CONTROL_VALUE
        );
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_rejects_non_image_response_without_linking_it(pool: SqlitePool) {
        let image_server = image_server(b"not an image", "text/plain").await;
        let episode = insert_episode(&pool, image_server.url()).await;
        let state = app_state(pool.clone());

        let error = get_cover(
            State(state.clone()),
            Path(episode.id.to_string()),
            HeaderMap::new(),
        )
        .await
        .expect_err("non-image response should be rejected");
        assert!(matches!(error, ApiError::Internal(_)));

        let stored_episode = state
            .db
            .episode
            .select(episode.sn)
            .await
            .expect("stored episode query should succeed")
            .expect("stored episode should exist");
        assert_eq!(stored_episode.cover_id, None);

        let cover_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM cover_image")
            .fetch_one(&pool)
            .await
            .expect("cover count query should succeed");
        assert_eq!(cover_count, 0);
        assert_eq!(image_server.request_count(), 1);
    }
}
