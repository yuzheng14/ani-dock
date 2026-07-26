use crate::constant::API_ORIGIN;

use serde_json::Value;
use wreq::header::REFERER;

use crate::{
    request::{RequestClient, anime_video::Video, common::CommonResponseBody},
    util::get_referer,
};

pub async fn get_anime_video_result_from_sn(
    sn: u32,
    request_client: &RequestClient,
) -> Result<Result<Video, Value>, wreq::Error> {
    let url = format!("{API_ORIGIN}/anime/v1/video.php?videoSn={sn}");
    let anime_video: CommonResponseBody<Video, Value> = request_client
        .get(&url, false)
        .header(REFERER, get_referer(sn))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(anime_video.into_result())
}
