use std::sync::{Arc, atomic::AtomicU32};

use ani_dock_core::{Config, EpisodeDownloader};
use futures::lock::Mutex;
use indexmap::IndexMap;
use tokio::sync::Semaphore;

use crate::CoreEpisode;

pub struct DonwloadCounter {
    current: AtomicU32,
    total: u32,
}

pub enum DownloadState {
    Pending,
    Downloading(DonwloadCounter),
    Downloaded,
}

pub struct Downloader {
    inner: EpisodeDownloader,
    state_map: IndexMap<u32, DownloadState>,
    semaphore: Semaphore,
    config: Arc<Mutex<Config>>,
}

impl Downloader {
    pub fn new(episode_downloader: EpisodeDownloader, config: Arc<Mutex<Config>>) -> Self {
        Self {
            inner: episode_downloader,
            state_map: IndexMap::new(),
            semaphore: Semaphore::new(1),
            config,
        }
    }

    pub fn schedule_download(&mut self, episode: CoreEpisode) {
        self.state_map.insert(episode.sn(), DownloadState::Pending);
    }

    fn download(&self) {
        todo!()
    }
}
