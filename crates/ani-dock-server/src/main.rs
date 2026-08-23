#[cfg(unix)]
use std::future::Future;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use ani_dock_core::{AnimeResolver, Config, Cookie, DeviceId, EpisodeDownloader, RequestClient};
use ani_dock_db::{
    get_conn_pool,
    repository::{
        AnimeRepository, CoverImageRepository, DownloadQueueRepository, EpisodeRepository,
    },
};
use ani_dock_server::{
    router::{AppState, DbRepository, get_app_router},
    service::{Downloader, Services},
};
use anyhow::Context;
use axum::serve;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[cfg(unix)]
async fn select_shutdown_signal<C, T>(ctrl_c: C, terminate: T) -> std::io::Result<&'static str>
where
    C: Future<Output = std::io::Result<()>>,
    T: Future<Output = std::io::Result<()>>,
{
    tokio::select! {
        result = ctrl_c => {
            result?;
            Ok("Ctrl-C")
        },
        result = terminate => {
            result?;
            Ok("SIGTERM")
        },
    }
}

async fn shutdown_signal() -> std::io::Result<&'static str> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        select_shutdown_signal(tokio::signal::ctrl_c(), async move {
            terminate.recv().await;
            Ok(())
        })
        .await
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        Ok("Ctrl-C")
    }
}

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
    let request_client = Arc::new(RequestClient::new(&config, cookie.clone())?);
    let config = Arc::new(Mutex::new(config));
    let device_id = DeviceId::default();

    let resolver = Arc::new(AnimeResolver::new(request_client.clone()));
    // TODO use notifier to change status
    let downloader = EpisodeDownloader::new(request_client.clone(), config.clone(), device_id);
    let shutdown = CancellationToken::new();
    let (download_service, download_worker) = Downloader::new(
        downloader,
        DownloadQueueRepository::new(pool.clone()),
        shutdown.clone(),
    );

    let state = AppState {
        db: DbRepository {
            anime: AnimeRepository::new(pool.clone()),
            episode: EpisodeRepository::new(pool.clone()),
            download_queue: DownloadQueueRepository::new(pool.clone()),
            cover_image: CoverImageRepository::new(pool.clone()),
        },
        resolver,
        services: Services {
            download: download_service,
        },
        shutdown: shutdown.clone(),
        config,
        cookie,
        request_client,
    };

    let server_result: Result<(), Box<dyn std::error::Error>> = async {
        let restored = state
            .services
            .download
            .restore_pending_downloads(&state.db.anime)
            .await
            .context("恢复待下载队列失败")?;
        tracing::info!(restored, "待下载队列恢复完成");

        let app = get_app_router(state);

        let host = std::env::var("ANI_DOCK_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = std::env::var("ANI_DOCK_PORT")
            .unwrap_or_else(|_| "6789".into())
            .parse::<u16>()?;

        let listener = tokio::net::TcpListener::bind((host.as_str(), port))
            .await
            .context("could not start server")?;
        tracing::info!(host = host, port = port, "server started");

        let shutdown_on_signal = shutdown.clone();
        serve(listener, app)
            .with_graceful_shutdown(async move {
                match shutdown_signal().await {
                    Ok(signal) => tracing::info!(signal, "收到关闭信号"),
                    Err(error) => tracing::error!(%error, "监听关闭信号失败"),
                }
                shutdown_on_signal.cancel();
            })
            .await?;

        Ok(())
    }
    .await;

    // Also stop background work if serving fails before a shutdown signal is received.
    shutdown.cancel();
    let worker_result = download_worker
        .wait()
        .await
        .context("等待下载 worker 退出失败");
    pool.close().await;

    server_result?;
    worker_result?;

    tracing::info!("服务器已安全关闭");
    std::io::stdout().flush().context("刷新标准输出失败")?;
    std::io::stderr().flush().context("刷新标准错误失败")?;

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

#[cfg(all(test, unix))]
mod tests {
    use std::{future, io};

    use super::select_shutdown_signal;

    #[tokio::test]
    async fn selects_ctrl_c_shutdown_signal() {
        let signal =
            select_shutdown_signal(future::ready(Ok(())), future::pending::<io::Result<()>>())
                .await
                .expect("Ctrl-C signal should be selected");

        assert_eq!(signal, "Ctrl-C");
    }

    #[tokio::test]
    async fn selects_sigterm_shutdown_signal() {
        let signal =
            select_shutdown_signal(future::pending::<io::Result<()>>(), future::ready(Ok(())))
                .await
                .expect("SIGTERM signal should be selected");

        assert_eq!(signal, "SIGTERM");
    }
}
