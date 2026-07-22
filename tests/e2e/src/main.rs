use ani_gamer_pro::{Anime, Config, Cookie, DeviceId, RequestClient};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};
use tokio::fs;

fn main() {
    println!("Hello, world!");
}

#[tokio::test]
async fn download_3499() -> Result<(), Box<dyn Error>> {
    let cookie_string = fs::read_to_string("./cookie.test.txt").await?;

    let device_id = DeviceId::default();
    let config = Arc::new(Mutex::new(Config::default()));
    let cookie = Cookie::new(cookie_string);
    let request_client = Arc::new(RequestClient::new(&config.lock().unwrap(), &cookie)?);

    let anime = Anime::from_episode_sn(3499, device_id, request_client, config).await?;

    anime
        .episodes()
        .first()
        .unwrap()
        .1
        .first()
        .unwrap()
        .download()
        .await?;

    Ok(())
}
