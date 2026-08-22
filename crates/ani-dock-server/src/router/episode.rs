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
