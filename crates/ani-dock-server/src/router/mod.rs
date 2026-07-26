use std::sync::Arc;

use ani_dock_core::{AnimeResolver, EpisodeDownloader};
use ani_dock_db::repository::AnimeRepository;
use axum::{Router, routing::get};

use crate::router::{
    anime::{import_anime, select_animes},
    health::health,
};

mod anime;
mod health;

#[derive(Debug, Clone)]
pub struct DbRepository {
    pub anime: AnimeRepository,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: DbRepository,
    pub resolver: Arc<AnimeResolver>,
    pub downloader: Arc<EpisodeDownloader>,
}

pub fn get_app_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/animes", get(select_animes).post(import_anime))
        .with_state(app_state)
}
