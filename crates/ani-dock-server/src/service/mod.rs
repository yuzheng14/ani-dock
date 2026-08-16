use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ani_dock_core::{
    DownloadStatusNotifier, EpisodeDownloadError, EpisodeDownloadEvent, EpisodeDownloader,
};
use ani_dock_db::repository::DownloadQueueRepository;
use indexmap::{IndexMap, map::Entry};
use tokio::{
    sync::{Semaphore, broadcast},
    time,
};

use crate::CoreEpisode;

#[derive(Debug, Clone)]
pub struct Services {
    pub download: Downloader,
}

pub type DownloadState = Result<EpisodeDownloadEvent, Arc<EpisodeDownloadError>>;

pub type StateMap = IndexMap<u32, DownloadState>;

#[derive(Debug, Clone)]
pub struct Downloader {
    inner: EpisodeDownloader,
    state_map: Arc<Mutex<StateMap>>,
    semaphore: Arc<Semaphore>,
    queue_repo: DownloadQueueRepository,
    tx: broadcast::Sender<DownloadStatus>,
}

impl Downloader {
    pub fn new(episode_downloader: EpisodeDownloader, repo: DownloadQueueRepository) -> Self {
        let (tx, _) = broadcast::channel(128);
        Self {
            inner: episode_downloader,
            state_map: Arc::new(Mutex::new(IndexMap::new())),
            semaphore: Arc::new(Semaphore::new(1)),
            queue_repo: repo,
            tx,
        }
    }

    /// This only means download task has been scheduled, not completed
    pub fn schedule_download(&self, episode: CoreEpisode) {
        let sn = episode.sn();
        {
            let mut state_map = self.state_map.lock().unwrap();

            // A non-error state means the episode is already queued or completed.
            // An error state may be replaced with Pending to retry the download.
            if state_map.get(&sn).is_some_and(|status| !status.is_err()) {
                return;
            }

            state_map.insert(sn, Ok(EpisodeDownloadEvent::Pending));
            let _ = self.tx.send(DownloadStatus {
                sn,
                state: Ok(EpisodeDownloadEvent::Pending),
            });
        }

        let this = self.clone();
        tokio::spawn(async move {
            let lock = this.semaphore.acquire().await;
            if lock.is_err() {
                tracing::error!("异常情况，获取下载器的并发锁失败");
                this.state_map
                    .lock()
                    .unwrap()
                    .insert(sn, Err(Arc::new(EpisodeDownloadError::AcquireSemaphore)));
                let _ = this.tx.send(DownloadStatus {
                    sn,
                    state: Err(Arc::new(EpisodeDownloadError::AcquireSemaphore)),
                });
                return;
            }

            // Model human viewing behavior by starting downloads at least 24 minutes apart.
            // If a download takes longer, the next task may start immediately after it finishes.
            let cooldown = time::sleep(Duration::from_secs(24 * 60));

            let status_map = this.state_map.clone();
            let tx = this.tx.clone();
            let notifier = DownloadStatusNotifier::new(move |event| {
                status_map.lock().unwrap().insert(sn, Ok(event.clone()));
                let _ = tx.send(DownloadStatus {
                    sn,
                    state: Ok(event),
                });
            });

            let download_result = this.inner.download(&episode, notifier).await;
            if let Err(err) = download_result {
                let error = Arc::new(err);
                this.state_map
                    .lock()
                    .unwrap()
                    .insert(sn, Err(error.clone()));
                let _ = this.tx.send(DownloadStatus {
                    sn,
                    state: Err(error.clone()),
                });
                tracing::error!(error = %error, sn = %sn, "下载发生错误")
            } else {
                if let Err(err) = this.queue_repo.mark_downloaded(sn).await {
                    tracing::error!(error = %err, "标记下载完成失败")
                }
            }

            cooldown.await;
        });
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use ani_dock_core::{Config, Cookie, DeviceId, RequestClient};
    use tokio::sync::Barrier;

    use super::*;

    async fn test_downloader() -> Downloader {
        let config = Config {
            proxy: Some("http://127.0.0.1:1".to_string()),
            ..Config::default()
        };
        let request_client = Arc::new(RequestClient::new(&config, Cookie::default()).unwrap());
        let config = Arc::new(Mutex::new(config));
        let inner = EpisodeDownloader::new(request_client, config, DeviceId::default());
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");

        Downloader::new(inner, DownloadQueueRepository::new(pool))
    }

    async fn wait_for_background_tasks_to_finish(downloader: &Downloader) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while Arc::strong_count(&downloader.state_map) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("background download tasks should finish");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_requests_schedule_only_one_download() {
        const REQUEST_COUNT: usize = 32;
        const SN: u32 = 12_345;

        let downloader = test_downloader().await;
        let permit = downloader.semaphore.acquire().await.unwrap();
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
            Arc::strong_count(&downloader.state_map),
            2,
            "exactly one background download task should be waiting"
        );

        downloader.semaphore.close();
        drop(permit);
        wait_for_background_tasks_to_finish(&downloader).await;
    }

    #[tokio::test]
    async fn failed_download_can_be_scheduled_again() {
        const SN: u32 = 23_456;

        let downloader = test_downloader().await;
        downloader
            .state_map
            .lock()
            .unwrap()
            .insert(SN, Err(Arc::new(EpisodeDownloadError::AcquireSemaphore)));
        let permit = downloader.semaphore.acquire().await.unwrap();

        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        assert!(matches!(
            downloader.state_map.lock().unwrap().get(&SN),
            Some(Ok(EpisodeDownloadEvent::Pending))
        ));
        assert_eq!(
            Arc::strong_count(&downloader.state_map),
            2,
            "retry should create one background download task"
        );

        downloader.semaphore.close();
        drop(permit);
        wait_for_background_tasks_to_finish(&downloader).await;
    }

    #[tokio::test]
    async fn completed_download_is_not_scheduled_again() {
        const SN: u32 = 34_567;

        let downloader = test_downloader().await;
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
        assert_eq!(
            Arc::strong_count(&downloader.state_map),
            1,
            "completed episode should not create a background download task"
        );
    }

    #[tokio::test]
    async fn closed_semaphore_is_reported_as_download_error() {
        const SN: u32 = 45_678;

        let downloader = test_downloader().await;
        downloader.semaphore.close();
        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let acquire_failed = matches!(
                    downloader.state_map.lock().unwrap().get(&SN),
                    Some(Err(error))
                        if matches!(error.as_ref(), EpisodeDownloadError::AcquireSemaphore)
                );
                if acquire_failed {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("closed semaphore should fail the scheduled download");
    }

    #[tokio::test]
    async fn failed_download_holds_permit_during_cooldown() {
        const SN: u32 = 56_789;

        let downloader = test_downloader().await;
        downloader.schedule_download(CoreEpisode::new(SN, 1, ""));

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let download_failed = downloader
                    .state_map
                    .lock()
                    .unwrap()
                    .get(&SN)
                    .is_some_and(Result::is_err);
                if download_failed {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("test proxy should make the download fail");

        assert_eq!(
            downloader.semaphore.available_permits(),
            0,
            "download permit should stay held until the cooldown ends"
        );
    }

    #[tokio::test]
    async fn subscribe_receives_broadcast_of_pending_state() {
        const SN: u32 = 67_890;

        let downloader = test_downloader().await;
        let permit = downloader.semaphore.acquire().await.unwrap();
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

        downloader.semaphore.close();
        drop(permit);
        wait_for_background_tasks_to_finish(&downloader).await;
    }
}
