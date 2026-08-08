use ani_dock_db::model::Episode;
use axum::{Json, extract::State, http::StatusCode};
use futures::future::try_join_all;

use crate::{ApiError, ApiResult, CoreEpisode, router::AppState};

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
