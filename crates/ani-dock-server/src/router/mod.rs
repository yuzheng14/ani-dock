use axum::{Router, routing::get};

use crate::router::health::health;

mod health;

pub fn get_router() -> Router {
    Router::new().route("/health", get(health))
}
