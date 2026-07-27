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

    todo!();

    Ok(StatusCode::ACCEPTED)
}
