use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ani_dock_core::{
    DownloadStatusNotifier, EpisodeDownloadError, EpisodeDownloadEvent, EpisodeDownloader,
    RequestClient, utils::get_referer,
};
use ani_dock_db::{
    input::CreateCoverImage,
    model::CoverImage,
    repository::{AnimeRepository, CoverImageRepository, DbResult, DownloadQueueRepository},
};
use anyhow::Context;
use indexmap::{IndexMap, map::Entry};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time,
};
use tokio_util::sync::CancellationToken;
use wreq::header::REFERER;

use crate::CoreEpisode;

#[derive(Debug, Clone)]
pub struct Services {
    pub download: Downloader,
}

pub type DownloadState = Result<EpisodeDownloadEvent, Arc<EpisodeDownloadError>>;

pub type StateMap = IndexMap<u32, DownloadState>;

struct DownloadWorker {
    inner: EpisodeDownloader,
    queue_rx: mpsc::UnboundedReceiver<CoreEpisode>,
    shutdown: CancellationToken,
    state_map: Arc<Mutex<StateMap>>,
    queue_repo: DownloadQueueRepository,
    tx: broadcast::Sender<DownloadStatus>,
}

impl DownloadWorker {
    async fn run(mut self) {
        loop {
            let episode = tokio::select! {
                // Once shutdown has been requested, queued downloads must stay pending instead
                // of racing the shutdown notification and starting new work.
                biased;
                () = self.shutdown.cancelled() => break,
                episode = self.queue_rx.recv() => {
                    let Some(episode) = episode else {
                        break;
                    };
                    episode
                }
            };

            if !self.download_one(episode).await {
                break;
            }
        }

        tracing::info!("下载任务已停止");
    }

    /// actual download
    ///
    /// Returns whether the worker should receive another queued download.
    async fn download_one(&self, episode: CoreEpisode) -> bool {
        let sn = episode.sn();
        // Model human viewing behavior by starting downloads at least 24 minutes apart.
        // If a download takes longer, the next task may start immediately after it finishes.
        let cooldown = time::sleep(Duration::from_secs(24 * 60));
        tokio::pin!(cooldown);

        let status_map = self.state_map.clone();
        let tx = self.tx.clone();
        let notifier = DownloadStatusNotifier::new(move |event| {
            status_map.lock().unwrap().insert(sn, Ok(event.clone()));
            let _ = tx.send(DownloadStatus {
                sn,
                state: Ok(event),
            });
        });

        let download_result = self.inner.download(&episode, notifier).await;
        if let Err(err) = download_result {
            let error = Arc::new(err);
            self.state_map
                .lock()
                .unwrap()
                .insert(sn, Err(error.clone()));
            let _ = self.tx.send(DownloadStatus {
                sn,
                state: Err(error.clone()),
            });
            tracing::error!(error = %error, sn = %sn, "下载发生错误")
        } else {
            if let Err(err) = self.queue_repo.mark_downloaded(sn).await {
                tracing::error!(error = %err, "标记下载完成失败")
            }
        }

        // Never cancel an in-flight download: finishing its finalization and database update
        // avoids leaving a partially copied output file. Shutdown does, however, skip the
        // cooldown and prevents the next queued download from starting. Unstarted/failed rows
        // remain `downloaded = 0` and are restored from the persistent queue on the next start.
        if self.queue_rx.is_closed() {
            return false;
        }

        tokio::select! {
            biased;
            () = self.shutdown.cancelled() => false,
            () = &mut cooldown => true,
        }
    }
}

#[derive(Debug)]
struct DownloadLifecycle {
    worker_task: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug, Clone)]
pub struct Downloader {
    state_map: Arc<Mutex<StateMap>>,
    queue_tx: mpsc::UnboundedSender<CoreEpisode>,
    tx: broadcast::Sender<DownloadStatus>,
    lifecycle: Arc<DownloadLifecycle>,
}

impl Downloader {
    pub fn new(
        episode_downloader: EpisodeDownloader,
        repo: DownloadQueueRepository,
        shutdown: CancellationToken,
    ) -> Self {
        let (downloader, worker) = Self::build(episode_downloader, repo, shutdown);
        let worker_task = tokio::spawn(worker.run());
        downloader
            .lifecycle
            .worker_task
            .lock()
            .unwrap()
            .replace(worker_task);

        downloader
    }

    // This is kept separate from `new` only as a test seam: tests need access to an
    // unstarted worker so they can inspect the receive side of the queue directly,
    // without racing the worker or triggering real downloads and their cooldown.
    fn build(
        episode_downloader: EpisodeDownloader,
        repo: DownloadQueueRepository,
        shutdown: CancellationToken,
    ) -> (Self, DownloadWorker) {
        let (tx, _) = broadcast::channel(128);
        let (queue_tx, queue_rx) = mpsc::unbounded_channel();
        let state_map = Arc::new(Mutex::new(IndexMap::new()));
        let lifecycle = Arc::new(DownloadLifecycle {
            worker_task: Mutex::new(None),
        });

        let worker = DownloadWorker {
            inner: episode_downloader,
            state_map: state_map.clone(),
            queue_rx,
            shutdown,
            queue_repo: repo,
            tx: tx.clone(),
        };

        (
            Self {
                state_map,
                tx,
                queue_tx,
                lifecycle,
            },
            worker,
        )
    }

    /// Wait until the background download worker has finished its orderly shutdown.
    pub async fn wait_for_worker(&self) -> Result<(), tokio::task::JoinError> {
        let worker_task = self.lifecycle.worker_task.lock().unwrap().take();

        if let Some(worker_task) = worker_task {
            worker_task.await?;
        }

        Ok(())
    }

    /// Schedule an already-persisted download for the in-memory worker.
    ///
    /// Callers must insert the episode into `download_queue` before calling this method. That
    /// ordering ensures a task accepted concurrently with shutdown can be restored next time,
    /// even if the worker has already observed cancellation and does not receive it.
    pub fn schedule_download(&self, episode: CoreEpisode) {
        let sn = episode.sn();

        let mut state_map = self.state_map.lock().unwrap();

        // A non-error state means the episode is already queued or completed.
        // An error state may be replaced with Pending to retry the download.
        if state_map.get(&sn).is_some_and(|status| !status.is_err()) {
            return;
        }

        if let Err(err) = self.queue_tx.send(episode) {
            tracing::warn!(error = %err, "下载队列关闭");
            return;
        }

        state_map.insert(sn, Ok(EpisodeDownloadEvent::Pending));

        let _ = self.tx.send(DownloadStatus {
            sn,
            state: Ok(EpisodeDownloadEvent::Pending),
        });
    }

    pub async fn restore_pending_downloads(&self, repo: &AnimeRepository) -> DbResult<usize> {
        let undownloaded_animes = repo.select_by_download_status(false).await?;
        let mut restored = 0;

        for anime in undownloaded_animes {
            for (_, episodes) in anime.series {
                for episode in episodes {
                    if !self.exists(episode.sn) {
                        self.schedule_download(episode.into());
                        restored += 1;
                    }
                }
            }
        }

        Ok(restored)
    }

    pub fn exists(&self, sn: u32) -> bool {
        matches!(self.state_map.lock().unwrap().entry(sn), Entry::Occupied(_))
    }

    pub fn get_undownloaded_episodes_sn(&self) -> Vec<u32> {
        self.state_map
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(sn, state)| {
                if let Ok(event) = state
                    && matches!(event, EpisodeDownloadEvent::Completed)
                {
                    None
                } else {
                    Some(sn.to_owned())
                }
            })
            .collect()
    }

    pub fn is_error(&self, sn: u32) -> bool {
        matches!(self.state_map.lock().unwrap().get(&sn), Some(Err(_)))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DownloadStatus> {
        self.tx.subscribe()
    }

    pub fn state_snapshot(&self) -> Vec<DownloadStatus> {
        self.state_map
            .lock()
            .unwrap()
            .iter()
            .map(|(sn, state)| DownloadStatus {
                sn: sn.to_owned(),
                state: state.to_owned(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct DownloadStatus {
    pub sn: u32,
    pub state: DownloadState,
}

/// this will not update cover_id for anime or episode
pub async fn request_cover(
    request_client: &RequestClient,
    cover_image_repo: &CoverImageRepository,
    url: &str,
    sn: u32,
) -> anyhow::Result<CoverImage> {
    let cover_resp = request_client
        .get(url, false)
        .header(REFERER, get_referer(sn))
        .send()
        .await
        .context("请求封面错误")?
        .error_for_status()
        .context("封面 HTTP 状态错误")?;

    let mime_type = cover_resp
        .headers()
        .get("Content-Type")
        .cloned()
        .context("封面响应缺少 Content-Type")?
        .to_str()
        .context("封面响应头转换字符串失败")?
        .to_owned();

    if !mime_type.starts_with("image/") {
        return Err(anyhow::anyhow!(
            "服务端返回资源类型为非图片，当前为 {mime_type}"
        ));
    }

    let bytes = cover_resp.bytes().await.context("读取封面数据失败")?;

    let cover_image = cover_image_repo
        .save(CreateCoverImage {
            url: url.into(),
            bytes,
            mime_type,
        })
        .await
        .context("存储封面数据失败")?;

    Ok(cover_image)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use ani_dock_core::{Config, Cookie, DeviceId, RequestClient};
    use ani_dock_db::{
        input::{CreateAnime, CreateEpisode},
        repository::AnimeRepository,
    };
    use sqlx::SqlitePool;
    use tokio::sync::Barrier;

    use super::*;

    async fn test_downloader_parts() -> (Downloader, DownloadWorker) {
        test_downloader_parts_with_shutdown(CancellationToken::new()).await
    }

    async fn test_downloader_parts_with_shutdown(
        shutdown: CancellationToken,
    ) -> (Downloader, DownloadWorker) {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");

        test_downloader_parts_with_pool_and_shutdown(pool, shutdown)
    }

    fn test_downloader_parts_with_pool(pool: SqlitePool) -> (Downloader, DownloadWorker) {
        test_downloader_parts_with_pool_and_shutdown(pool, CancellationToken::new())
    }

    fn test_downloader_parts_with_pool_and_shutdown(
        pool: SqlitePool,
        shutdown: CancellationToken,
    ) -> (Downloader, DownloadWorker) {
        let config = Config {
            proxy: Some("http://127.0.0.1:1".to_string()),
            ..Config::default()
        };
        let request_client = Arc::new(RequestClient::new(&config, Cookie::default()).unwrap());
        let config = Arc::new(Mutex::new(config));
        let inner = EpisodeDownloader::new(request_client, config, DeviceId::default());

        Downloader::build(inner, DownloadQueueRepository::new(pool), shutdown)
    }

    fn assert_queue_empty(worker: &mut DownloadWorker) {
        assert!(matches!(
            worker.queue_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }

    #[sqlx::test(migrations = "../ani-dock-db/migrations")]
    async fn restore_pending_downloads_schedules_persisted_work_without_http_request(
        pool: SqlitePool,
    ) {
        const PENDING_SN: u32 = 12_345;
        const DOWNLOADED_SN: u32 = 23_456;

        let anime_repo = AnimeRepository::new(pool.clone());
        anime_repo
            .insert(CreateAnime {
                sn: PENDING_SN,
                cover: "https://example.com/anime.jpg".to_owned(),
                name: "测试动画".to_owned(),
                series: IndexMap::from([(
                    "本篇".to_owned(),
                    vec![
                        CreateEpisode {
                            sn: PENDING_SN,
                            cover: "https://example.com/pending.jpg".to_owned(),
                            episode: 1,
                        },
                        CreateEpisode {
                            sn: DOWNLOADED_SN,
                            cover: "https://example.com/downloaded.jpg".to_owned(),
                            episode: 2,
                        },
                    ],
                )]),
            })
            .await
            .expect("anime fixture should be inserted");

        let queue_repo = DownloadQueueRepository::new(pool.clone());
        for (sn, episode) in [(PENDING_SN, 1), (DOWNLOADED_SN, 2)] {
            assert!(
                queue_repo
                    .insert_by_episode_or_ignore(CoreEpisode::new(sn, episode, ""))
                    .await
                    .expect("episode should be queued")
            );
        }
        queue_repo
            .mark_downloaded(DOWNLOADED_SN)
            .await
            .expect("downloaded fixture should be marked completed");

        let (downloader, mut worker) = test_downloader_parts_with_pool(pool);

        assert_eq!(
            downloader
                .restore_pending_downloads(&anime_repo)
                .await
                .expect("pending downloads should be restored"),
            1
        );
        assert_eq!(
            worker
                .queue_rx
                .try_recv()
                .expect("pending download should be queued")
                .sn(),
            PENDING_SN
        );
        assert_queue_empty(&mut worker);

        assert_eq!(
            downloader
                .restore_pending_downloads(&anime_repo)
                .await
                .expect("repeated restoration should succeed"),
            0
        );
        assert_queue_empty(&mut worker);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_requests_schedule_only_one_download() {
        const REQUEST_COUNT: usize = 32;
        const SN: u32 = 12_345;

        let (downloader, mut worker) = test_downloader_parts().await;
        let barrier = Arc::new(Barrier::new(REQUEST_COUNT + 1));
        let mut requests = Vec::with_capacity(REQUEST_COUNT);

        for _ in 0..REQUEST_COUNT {
            let downloader = downloader.clone();
            let barrier = barrier.clone();
            requests.push(tokio::spawn(async move {
                barrier.wait().await;
                downloader.schedule_download(CoreEpisode::new(SN, 1, ""));
            }));
        }

        barrier.wait().await;
        for request in requests {
            request.await.unwrap();
        }

        assert!(matches!(
            downloader.state_map.lock().unwrap().get(&SN),
            Some(Ok(EpisodeDownloadEvent::Pending))
        ));
        assert_eq!(
            worker
                .queue_rx
                .try_recv()
                .expect("one download should be queued")
                .sn(),
            SN
        );
        assert_queue_empty(&mut worker);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_downloads_follow_the_recorded_schedule_order() {
        const DOWNLOAD_COUNT: u32 = 32;

        let (downloader, mut worker) = test_downloader_parts().await;
        let barrier = Arc::new(Barrier::new(DOWNLOAD_COUNT as usize + 1));
        let mut requests = Vec::with_capacity(DOWNLOAD_COUNT as usize);

        for sn in 1..=DOWNLOAD_COUNT {
            let downloader = downloader.clone();
            let barrier = barrier.clone();
            requests.push(tokio::spawn(async move {
                barrier.wait().await;
                downloader.schedule_download(CoreEpisode::new(sn, sn, ""));
            }));
        }

        barrier.wait().await;
        for request in requests {
            request.await.unwrap();
        }

        let recorded_order = downloader
            .state_map
            .lock()
            .unwrap()
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let queued_order = (0..DOWNLOAD_COUNT)
            .map(|_| {
                worker
                    .queue_rx
                    .try_recv()
                    .expect("scheduled download should be queued")
                    .sn()
            })
            .collect::<Vec<_>>();

        assert_eq!(queued_order, recorded_order);
        assert_queue_empty(&mut worker);
    }

    #[tokio::test]
    async fn failed_download_can_be_scheduled_again() {
        const SN: u32 = 23_456;

        let (downloader, mut worker) = test_downloader_parts().await;
        downloader.state_map.lock().unwrap().insert(
            SN,
            Err(Arc::new(EpisodeDownloadError::Api(
                "previous failure".to_owned(),
            ))),
        );

        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        assert!(matches!(
            downloader.state_map.lock().unwrap().get(&SN),
            Some(Ok(EpisodeDownloadEvent::Pending))
        ));
        assert_eq!(
            worker
                .queue_rx
                .try_recv()
                .expect("retry should be queued")
                .sn(),
            SN
        );
        assert_queue_empty(&mut worker);
    }

    #[tokio::test]
    async fn completed_download_is_not_scheduled_again() {
        const SN: u32 = 34_567;

        let (downloader, mut worker) = test_downloader_parts().await;
        downloader
            .state_map
            .lock()
            .unwrap()
            .insert(SN, Ok(EpisodeDownloadEvent::Completed));

        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        assert!(matches!(
            downloader.state_map.lock().unwrap().get(&SN),
            Some(Ok(EpisodeDownloadEvent::Completed))
        ));
        assert_queue_empty(&mut worker);
    }

    #[tokio::test]
    async fn closed_queue_does_not_mark_download_as_pending() {
        const SN: u32 = 45_678;

        let (downloader, worker) = test_downloader_parts().await;
        let mut rx = downloader.subscribe();
        drop(worker);

        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        assert!(!downloader.state_map.lock().unwrap().contains_key(&SN));
        assert!(matches!(
            rx.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn downloads_are_queued_in_schedule_order() {
        const SNS: [u32; 3] = [56_789, 12_345, 34_567];

        let (downloader, mut worker) = test_downloader_parts().await;

        for (index, sn) in SNS.into_iter().enumerate() {
            downloader.schedule_download(CoreEpisode::new(sn, index as u32 + 1, ""));
        }

        let queued_sns = (0..SNS.len())
            .map(|_| {
                worker
                    .queue_rx
                    .try_recv()
                    .expect("scheduled download should be queued")
                    .sn()
            })
            .collect::<Vec<_>>();

        assert_eq!(queued_sns, SNS);
        assert_queue_empty(&mut worker);
    }

    #[tokio::test]
    async fn worker_stops_after_all_downloaders_are_dropped() {
        let (downloader, worker) = test_downloader_parts().await;
        let worker_task = tokio::spawn(worker.run());

        drop(downloader);

        time::timeout(Duration::from_secs(1), worker_task)
            .await
            .expect("worker should stop after all queue senders are dropped")
            .expect("worker task should not panic");
    }

    #[tokio::test]
    async fn shutdown_stops_worker_without_starting_a_late_download() {
        const SN: u32 = 78_901;

        let shutdown = CancellationToken::new();
        let (downloader, worker) = test_downloader_parts_with_shutdown(shutdown.clone()).await;
        shutdown.cancel();
        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        time::timeout(Duration::from_secs(1), worker.run())
            .await
            .expect("worker should stop promptly when shutdown is requested");

        assert!(matches!(
            downloader.state_map.lock().unwrap().get(&SN),
            Some(Ok(EpisodeDownloadEvent::Pending))
        ));
    }

    #[tokio::test]
    async fn subscribe_receives_broadcast_of_pending_state() {
        const SN: u32 = 67_890;

        let (downloader, mut worker) = test_downloader_parts().await;
        let mut rx = downloader.subscribe();

        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        let status = rx
            .recv()
            .await
            .expect("schedule_download should broadcast a pending event");
        assert_eq!(status.sn, SN);
        assert!(matches!(status.state, Ok(EpisodeDownloadEvent::Pending)));

        let snapshot = downloader.state_snapshot();
        assert!(
            snapshot
                .iter()
                .any(|s| { s.sn == SN && matches!(s.state, Ok(EpisodeDownloadEvent::Pending)) })
        );

        assert_eq!(
            worker
                .queue_rx
                .try_recv()
                .expect("pending download should be queued")
                .sn(),
            SN
        );
        assert_queue_empty(&mut worker);
    }
}
