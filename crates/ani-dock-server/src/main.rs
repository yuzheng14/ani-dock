use std::sync::{Arc, Mutex};

use ani_dock_core::{AnimeResolver, Config, Cookie, DeviceId, EpisodeDownloader, RequestClient};
use ani_dock_db::{
    get_conn_pool,
    repository::{AnimeRepository, DownloadQueueRepository, EpisodeRepository},
};
use ani_dock_server::{
    router::{AppState, DbRepository, get_app_router},
    service::{Downloader, Services},
};
use axum::serve;
use tracing_subscriber::EnvFilter;

async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_line_number(true)
        .init();

    let pool = get_conn_pool().await?;
    let cookie = Cookie::read_cookie().await?;
    let config = Config::read_config().await?;
    let request_client = Arc::new(RequestClient::new(&config, cookie)?);
    let config = Arc::new(Mutex::new(config));
    let device_id = DeviceId::default();

    let resolver = Arc::new(AnimeResolver::new(request_client.clone()));
    // TODO use notifier to change status
    let downloader = EpisodeDownloader::new(request_client, config, device_id);

    let state = AppState {
        db: DbRepository {
            anime: AnimeRepository::new(pool.clone()),
            episode: EpisodeRepository::new(pool.clone()),
            download_queue: DownloadQueueRepository::new(pool.clone()),
        },
        resolver,
        services: Services {
            download: Downloader::new(downloader, DownloadQueueRepository::new(pool.clone())),
        },
    };

    let app = get_app_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:6789")
        .await
        .expect("could not start server");
    tracing::info!("server started, listener at: 127.0.0.1:6789");

    serve(listener, app).await.expect("server serve error");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = start_server().await;

    if let Err(error) = &result {
        tracing::error!(error = %error, "启动服务器发生错误");
    }

    result
}
