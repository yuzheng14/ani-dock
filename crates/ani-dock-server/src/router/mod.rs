use std::sync::Arc;

use ani_dock_core::AnimeResolver;
use ani_dock_db::repository::{AnimeRepository, DownloadQueueRepository, EpisodeRepository};
use axum::{
    Router,
    routing::{get, post, put},
};

use crate::{
    router::{
        anime::{import_anime, select_animes},
        episode::{download, get_undownload_episodes, restore_download_list},
        health::health,
    },
    service::Services,
};

mod anime;
mod episode;
mod health;

#[derive(Debug, Clone)]
pub struct DbRepository {
    pub anime: AnimeRepository,
    pub episode: EpisodeRepository,
    pub download_queue: DownloadQueueRepository,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: DbRepository,
    pub resolver: Arc<AnimeResolver>,
    pub services: Services,
}

pub fn get_app_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/animes", get(select_animes).post(import_anime))
        .route("/episodes/download", put(download))
        .route("/episodes/undownloaded", get(get_undownload_episodes))
        .route("/episodes/download/restore", post(restore_download_list))
        .with_state(app_state)
}
