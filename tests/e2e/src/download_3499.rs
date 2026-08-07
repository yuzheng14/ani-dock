use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use ani_dock_core::{
    AnimeResolver, Config, Cookie, DeviceId, DownloadStatusNotifier, EpisodeDownloader,
    RequestClient,
};
use tokio::fs;

#[tokio::test]
async fn download_3499() -> Result<(), Box<dyn Error>> {
    crate::common::init_test_tracing();

    let cookie_string = fs::read_to_string("./cookie.test.txt").await?;

    let device_id = DeviceId::default();
    let config = Arc::new(Mutex::new(Config::default()));
    let cookie = Cookie::new(cookie_string);
    let request_client = Arc::new(RequestClient::new(&config.lock().unwrap(), cookie)?);

    let resolver = AnimeResolver::new(request_client.clone());
    let downloader = EpisodeDownloader::new(request_client, config, device_id);

    let anime = resolver.from_episode_sn(3499).await?;

    downloader
        .download(
            anime.series().first().unwrap().1.first().unwrap(),
            DownloadStatusNotifier::default(),
        )
        .await?;

    Ok(())
}
