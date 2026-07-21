use thiserror::Error;

use crate::{ffmpeg::FFmpegError, request::token::TokenError};

#[derive(Debug, Error)]
pub enum AnimeDownloadError {
    #[error("parse url error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("http request error: {0}")]
    WreqError(#[from] wreq::Error),
    #[error("device id didn't exist")]
    DeviceIdDidNotExist,
    #[error("verify user permission error: {0}")]
    VerifyUserPermissionError(#[from] TokenError),
    #[error("has set only use vip, but current account is not vip")]
    SetOnlyVipButNot,
    #[error("waiting ads timeout")]
    WaitAdsTimeout,
    #[error("request error: {0}")]
    Request(String),
    #[error("parse m3u8 play list error: {0}")]
    M3u8Parse(nom::error::Error<String>),
    #[error("running ffmpeg error: {0}")]
    FFmpeg(#[from] FFmpegError),
    #[error("manipulate file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    EpisodeDetailBuild(#[from] EpisodeDetailBuildError),
    #[error("{0}")]
    Plain(String),
}

impl From<String> for AnimeDownloadError {
    fn from(value: String) -> Self {
        Self::Request(value)
    }
}

impl From<nom::error::Error<&[u8]>> for AnimeDownloadError {
    fn from(value: nom::error::Error<&[u8]>) -> Self {
        Self::M3u8Parse(nom::error::Error::new(
            String::from_utf8_lossy(value.input).into_owned(),
            value.code,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EpisodeDetailBuildError {
    #[error("requst error when build episode detail: {0}")]
    Request(#[from] wreq::Error),
    #[error("{0}")]
    Plain(String),
}
