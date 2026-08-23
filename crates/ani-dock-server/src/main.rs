use std::{
    future::Future,
    io::{self, Write},
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
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShutdownSignal {
    CtrlC,
    Terminate,
}

async fn wait_for_shutdown_signal<C, T>(ctrl_c: C, terminate: T) -> io::Result<ShutdownSignal>
where
    C: Future<Output = io::Result<()>>,
    T: Future<Output = io::Result<()>>,
{
    tokio::pin!(ctrl_c);
    tokio::pin!(terminate);

    tokio::select! {
        result = &mut ctrl_c => {
            result?;
            Ok(ShutdownSignal::CtrlC)
        }
        result = &mut terminate => {
            result?;
            Ok(ShutdownSignal::Terminate)
        }
    }
}

async fn shutdown_signal() -> io::Result<ShutdownSignal> {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        signal.recv().await.ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "SIGTERM signal stream closed")
        })?;
        Ok(())
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<io::Result<()>>();

    wait_for_shutdown_signal(ctrl_c, terminate).await
}

fn flush_logs() {
    if let Err(error) = io::stderr().lock().flush() {
        eprintln!("刷新日志输出失败：{error}");
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

    let state = AppState {
        db: DbRepository {
            anime: AnimeRepository::new(pool.clone()),
            episode: EpisodeRepository::new(pool.clone()),
            download_queue: DownloadQueueRepository::new(pool.clone()),
            cover_image: CoverImageRepository::new(pool.clone()),
        },
        resolver,
        services: Services {
            download: Downloader::new(downloader, DownloadQueueRepository::new(pool.clone())),
        },
        config,
        cookie,
        request_client,
    };

    let restored = state
        .services
        .download
        .restore_pending_downloads(&state.db.anime)
        .await
        .context("恢复待下载队列失败")?;
    tracing::info!(restored, "待下载队列恢复完成");

    let downloader = state.services.download.clone();
    let shutdown_downloader = downloader.clone();
    let app = get_app_router(state);

    let host = std::env::var("ANI_DOCK_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("ANI_DOCK_PORT")
        .unwrap_or_else(|_| "6789".into())
        .parse::<u16>()?;

    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .expect("could not start server");
    tracing::info!(host = host, port = port, "server started");

    let server_result = serve(listener, app)
        .with_graceful_shutdown(async move {
            match shutdown_signal().await {
                Ok(signal) => tracing::info!(?signal, "收到关闭信号，开始优雅关闭"),
                Err(error) => {
                    tracing::error!(%error, "监听关闭信号失败，开始关闭服务");
                }
            }

            shutdown_downloader.begin_shutdown();
        })
        .await;

    // Also stop the worker if the HTTP server exits because of an error instead of a signal.
    downloader.begin_shutdown();
    let worker_result = downloader.wait_for_worker().await;
    pool.close().await;

    server_result?;
    worker_result.context("等待下载任务关闭失败")?;
    tracing::info!("服务器已关闭");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = start_server().await;

    if let Err(error) = &result {
        tracing::error!(error = %error, "启动服务器发生错误");
    }

    flush_logs();

    result
}

#[cfg(test)]
mod tests {
    use std::future::{pending, ready};

    use super::*;

    #[tokio::test]
    async fn ctrl_c_triggers_shutdown() {
        let signal = wait_for_shutdown_signal(ready(Ok(())), pending())
            .await
            .expect("Ctrl-C should trigger shutdown");

        assert_eq!(signal, ShutdownSignal::CtrlC);
    }

    #[tokio::test]
    async fn sigterm_triggers_shutdown() {
        let signal = wait_for_shutdown_signal(pending(), ready(Ok(())))
            .await
            .expect("SIGTERM should trigger shutdown");

        assert_eq!(signal, ShutdownSignal::Terminate);
    }

    #[tokio::test]
    async fn signal_listener_errors_are_propagated() {
        let error =
            wait_for_shutdown_signal(ready(Err(io::Error::other("listener failed"))), pending())
                .await
                .expect_err("signal listener errors should be propagated");

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }
}
