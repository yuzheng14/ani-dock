use std::sync::{Arc, Mutex};

use ani_dock_core::{AnimeResolver, Config, Cookie, RequestClient};
use ani_dock_db::repository::{
    AnimeRepository, CoverImageRepository, DownloadQueueRepository, EpisodeRepository,
};
use axum::{Router, http::StatusCode, routing::get};
use tokio_util::sync::CancellationToken;
#[cfg(not(debug_assertions))]
use tower_http::services::{ServeDir, ServeFile};

use crate::{router::health::health, service::Services};

mod anime;
mod cover;
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
    pub shutdown: CancellationToken,
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

#[cfg(test)]
pub(crate) mod test_helpers {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use ani_dock_core::{
        AnimeResolver, Config, Cookie, DeviceId, EpisodeDownloader, RequestClient,
    };
    use ani_dock_db::repository::{
        AnimeRepository, CoverImageRepository, DownloadQueueRepository, EpisodeRepository,
    };
    use axum::{Router, body::Bytes, http::header::CONTENT_TYPE, routing::get};
    use sqlx::SqlitePool;
    use tokio::{net::TcpListener, task::JoinHandle};
    use tokio_util::sync::CancellationToken;

    use super::{AppState, DbRepository};
    use crate::service::{Downloader, Services};

    pub(crate) fn app_state(pool: SqlitePool) -> AppState {
        app_state_with_config(pool, Config::default())
    }

    pub(crate) fn app_state_with_config(pool: SqlitePool, config: Config) -> AppState {
        let cookie = Cookie::default();
        let request_client = Arc::new(
            RequestClient::new(&config, cookie.clone()).expect("test request client should build"),
        );
        let config = Arc::new(Mutex::new(config));
        let episode_downloader =
            EpisodeDownloader::new(request_client.clone(), config.clone(), DeviceId::default());
        let shutdown = CancellationToken::new();

        AppState {
            shutdown: shutdown.clone(),
            db: DbRepository {
                anime: AnimeRepository::new(pool.clone()),
                episode: EpisodeRepository::new(pool.clone()),
                download_queue: DownloadQueueRepository::new(pool.clone()),
                cover_image: CoverImageRepository::new(pool.clone()),
            },
            resolver: Arc::new(AnimeResolver::new(request_client.clone())),
            services: Services {
                download: Downloader::new(
                    episode_downloader,
                    DownloadQueueRepository::new(pool),
                    shutdown.child_token(),
                ),
            },
            config,
            cookie,
            request_client,
        }
    }

    pub(crate) struct ImageServer {
        url: String,
        request_count: Arc<AtomicUsize>,
        task: JoinHandle<()>,
    }

    impl ImageServer {
        pub(crate) fn url(&self) -> &str {
            &self.url
        }

        pub(crate) fn request_count(&self) -> usize {
            self.request_count.load(Ordering::Relaxed)
        }
    }

    impl Drop for ImageServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    pub(crate) async fn image_server(bytes: &'static [u8], mime_type: &'static str) -> ImageServer {
        let request_count = Arc::new(AtomicUsize::new(0));
        let app = Router::new().route(
            "/cover",
            get({
                let request_count = request_count.clone();
                move || {
                    let request_count = request_count.clone();
                    async move {
                        request_count.fetch_add(1, Ordering::Relaxed);
                        ([(CONTENT_TYPE, mime_type)], Bytes::from_static(bytes))
                    }
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test image server should bind");
        let address = listener
            .local_addr()
            .expect("test image server should have a local address");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test image server should run");
        });

        ImageServer {
            url: format!("http://{address}/cover"),
            request_count,
            task,
        }
    }
}
