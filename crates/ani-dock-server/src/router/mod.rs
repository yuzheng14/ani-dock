use std::sync::{Arc, Mutex};

use ani_dock_core::{AnimeResolver, Config, Cookie};
use ani_dock_db::repository::{AnimeRepository, DownloadQueueRepository, EpisodeRepository};
use axum::{
    Router,
    http::StatusCode,
    routing::{get, post, put},
};
#[cfg(not(debug_assertions))]
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    router::{
        anime::{import_anime, select_animes},
        episode::{download, download_events, get_undownload_episodes, restore_download_list},
        health::health,
    },
    service::Services,
};

mod anime;
mod episode;
mod health;
mod settings;

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
    pub config: Arc<Mutex<Config>>,
    pub cookie: Cookie,
}

pub fn get_app_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .route("/health", get(health))
        .route("/animes", get(select_animes).post(import_anime))
        .route("/episodes/download", put(download))
        .route("/episodes/undownloaded", get(get_undownload_episodes))
        .route("/episodes/download/restore", post(restore_download_list))
        .route("/episodes/download/events", get(download_events))
        .nest("/settings", settings::router())
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(app_state);

    let router = Router::new().nest("/api", api_router);

    #[cfg(not(debug_assertions))]
    let router = router
        .fallback_service(ServeDir::new("./dist").fallback(ServeFile::new("./dist/index.html")));

    router
}
