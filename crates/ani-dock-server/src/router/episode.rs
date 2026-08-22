use std::convert::Infallible;

use ani_dock_db::{
    model::Episode,
    repository::{DbResult, EpisodeRepository},
};
use anyhow::Context;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderName, StatusCode},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post, put},
};
use futures::{Stream, StreamExt, future::try_join_all, stream};
use serde::Serialize;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use ts_rs::TS;
use wreq::header::CONTENT_TYPE;

use crate::{
    ApiError, ApiResult, CoreEpisode,
    router::AppState,
    service::{DownloadState, DownloadStatus, request_cover},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/download", put(download))
        .route("/undownloaded", get(get_undownload_episodes))
        .route("/download/restore", post(restore_download_list))
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

pub async fn restore_download_list(State(state): State<AppState>) -> ApiResult {
    let undownloaded_animes = state.db.anime.select_by_download_status(false).await?;

    undownloaded_animes.into_iter().for_each(|anime| {
        anime.series.into_iter().for_each(|(_, episodes)| {
            episodes.into_iter().for_each(|episode| {
                if !state.services.download.exists(episode.sn) {
                    state.services.download.schedule_download(episode.into())
                }
            })
        })
    });

    Ok(())
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

    let stream = stream::once(async { Ok::<_, Infallible>(snapshot) }).chain(updates);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn get_cover(
    State(state): State<AppState>,
    Path(id_or_sn): Path<String>,
) -> ApiResult<([(HeaderName, String); 1], Bytes)> {
    let episode = state
        .db
        .episode
        .select_one_by_id_or_sn(&id_or_sn)
        .await
        .context("查询剧集信息出错")?
        .ok_or(ApiError::NotFound)?;

    let (mime_type, bytes) = if let Some(cover_id) = episode.cover_id {
        let cover_image = state
            .db
            .cover_image
            .select_one(&cover_id.to_string())
            .await
            .context("查询剧集信息出错")?;

        (cover_image.mime_type, cover_image.bytes)
    } else if episode.cover.is_empty() {
        return Err(ApiError::NotFound);
    } else {
        let (mime_type, bytes, cover_id) = request_cover(
            &state.request_client,
            &state.db.cover_image,
            &episode.cover,
            episode.sn,
        )
        .await?;

        state
            .db
            .episode
            .update_cover_id(episode.id, cover_id)
            .await
            .context("更新剧集的封面资源引用出错")?;

        (mime_type, bytes)
    };

    Ok(([(CONTENT_TYPE, mime_type)], bytes))
}

#[cfg(test)]
mod cover_tests {
    use ani_dock_db::{
        input::{CreateAnime, CreateEpisode},
        repository::{AnimeRepository, EpisodeRepository},
    };
    use axum::extract::{Path, State};
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

        let (headers, bytes) = get_cover(State(state.clone()), Path(episode.id.to_string()))
            .await
            .expect("first cover request should succeed");
        assert_eq!(headers, [(CONTENT_TYPE, "image/png".to_owned())]);
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
        assert_eq!(image_server.request_count(), 1);
        drop(image_server);

        let (_, cached_bytes) = get_cover(State(state), Path(episode.sn.to_string()))
            .await
            .expect("cached cover request should succeed by episode sn");
        assert_eq!(cached_bytes.as_ref(), IMAGE_BYTES);
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_returns_not_found_when_episode_cover_is_empty(pool: SqlitePool) {
        let episode = insert_episode(&pool, "").await;
        let state = app_state(pool);

        let error = get_cover(State(state), Path(episode.id.to_string()))
            .await
            .expect_err("empty episode cover should return not found");

        assert!(matches!(error, ApiError::NotFound));
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn get_cover_rejects_non_image_response_without_linking_it(pool: SqlitePool) {
        let image_server = image_server(b"not an image", "text/plain").await;
        let episode = insert_episode(&pool, image_server.url()).await;
        let state = app_state(pool.clone());

        let error = get_cover(State(state.clone()), Path(episode.id.to_string()))
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
