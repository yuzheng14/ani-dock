use std::sync::{Arc, Mutex};

use ani_dock_core::{AnimeResolver, Config, Cookie, RequestClient};
use ani_dock_db::repository::{
    AnimeRepository, CoverImageRepository, DownloadQueueRepository, EpisodeRepository,
};
use axum::{Router, http::StatusCode, routing::get};
#[cfg(not(debug_assertions))]
use tower_http::services::{ServeDir, ServeFile};

use crate::{router::health::health, service::Services};

mod anime;
mod episode;
mod health;
mod settings;

#[derive(Debug, Clone)]
pub struct DbRepository {
    pub anime: AnimeRepository,
    pub episode: EpisodeRepository,
    pub download_queue: DownloadQueueRepository,
    pub cover_image: CoverImageRepository,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: DbRepository,
    pub resolver: Arc<AnimeResolver>,
    pub services: Services,
    pub config: Arc<Mutex<Config>>,
    pub cookie: Cookie,
    pub request_client: Arc<RequestClient>,
}

pub fn get_app_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .route("/health", get(health))
        .nest("/animes", anime::router())
        .nest("/episodes", episode::router())
        .nest("/settings", settings::router())
        .fallback(|| async { StatusCode::NOT_FOUND })
        .with_state(app_state);

    let router = Router::new().nest("/api", api_router);

    #[cfg(not(debug_assertions))]
    let router = router
        .fallback_service(ServeDir::new("./dist").fallback(ServeFile::new("./dist/index.html")));

    router
}
