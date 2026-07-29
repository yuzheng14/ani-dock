use std::sync::{Arc, Mutex};

use ani_dock_core::{
    Config, DownloadStatusNotifier, EpisodeDownloadError, EpisodeDownloadEvent, EpisodeDownloader,
};
use indexmap::IndexMap;
use tokio::sync::{Semaphore, mpsc};

use crate::CoreEpisode;

pub struct Services {
    download: Downloader,
}

#[derive(Clone)]
pub struct Downloader {
    inner: EpisodeDownloader,
    state_map: Arc<Mutex<IndexMap<u32, Result<EpisodeDownloadEvent, Arc<EpisodeDownloadError>>>>>,
    semaphore: Arc<Semaphore>,
    config: Arc<Mutex<Config>>,
}

impl Downloader {
    pub fn new(episode_downloader: EpisodeDownloader, config: Arc<Mutex<Config>>) -> Self {
        Self {
            inner: episode_downloader,
            state_map: Arc::new(Mutex::new(IndexMap::new())),
            semaphore: Arc::new(Semaphore::new(1)),
            config,
        }
    }

    pub fn schedule_download(&self, episode: CoreEpisode) {
        if self.state_map.lock().unwrap().contains_key(&episode.sn()) {
            return;
        }
        let sn = episode.sn();
        self.state_map
            .lock()
            .unwrap()
            .insert(sn, Ok(EpisodeDownloadEvent::Pending));

        let this = self.clone();
        tokio::spawn(async move {
            let lock = this.semaphore.acquire().await;
            if lock.is_err() {
                tracing::error!("异常情况，获取下载器的并发锁失败");
                this.state_map
                    .lock()
                    .unwrap()
                    .insert(sn, Err(Arc::new(EpisodeDownloadError::AcquireSemaphore)));
                // tx.send(Err(Arc::new(EpisodeDownloadError::AcquireSemaphore)))
                //     .unwrap();
                return;
            }

            let status_map = this.state_map.clone();
            let notifier = DownloadStatusNotifier::new(move |event| {
                // tx_cloned.send(Ok(event)).unwrap();
                status_map.lock().unwrap().insert(sn, Ok(event));
            });

            let download_result = this.inner.download(&episode, notifier).await;
            if let Err(err) = download_result {
                this.state_map
                    .lock()
                    .unwrap()
                    .insert(sn, Err(Arc::new(err)));
            }
        });
    }
}
