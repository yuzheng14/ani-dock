// pub(crate) mod anime_episode;
pub(crate) mod anime_resolver;
pub(crate) mod config;
pub mod constant;
pub(crate) mod cookie;
pub(crate) mod device_id;
pub(crate) mod episode_downloader;
pub(crate) mod ffmpeg;
pub(crate) mod model;
pub(crate) mod request;
pub(crate) mod sn_list;
pub(crate) mod util;

pub use anime_resolver::{AnimeResolveError, AnimeResolver};
pub use config::{Config, ConfigError, ConfigVersion, DownloadResolution, InternalConfig};
pub use cookie::{Cookie, CookieError};
pub use device_id::DeviceId;
pub use episode_downloader::{
    DownloadStatusNotifier, EpisodeDownloadError, EpisodeDownloadEvent, EpisodeDownloader,
};
pub use model::{anime::Anime, episode::Episode, episode_detail::EpisodeDetail};
pub use request::{RequestClient, RequestError};

pub mod error {

    pub use crate::anime_resolver::AnimeResolveError;
    pub use crate::config::ConfigError;
    pub use crate::cookie::CookieError;
    pub use crate::episode_downloader::EpisodeDownloadError;
    pub use crate::ffmpeg::FFmpegError;
    pub use crate::model::episode_detail::EpisodeDetailBuildError;
    pub use crate::request::RequestError;
    pub use crate::request::token::TokenError;
}

#[cfg(test)]
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
