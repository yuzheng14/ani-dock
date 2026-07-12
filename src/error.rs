use thiserror::Error;
use tokio::io;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("encounter io error when manipulate config file: {0}")]
    IO(#[from] io::Error),
    #[error("encounter toml serialize error when writing config file: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("encounter toml parse error when reading config file: {0}")]
    TomlDe(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum SnListError {
    #[error("encounter io error when manipulate sn list file: {0}")]
    IO(#[from] io::Error),
    #[error("encounter toml serialize error when writing sn list file: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("encounter toml parse error when reading sn list file: {0}")]
    TomlDe(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum CookieError {
    #[error("encounter io error when manipulate cookie file: {0}")]
    IO(#[from] io::Error),
    #[error("could not find cookie file")]
    NotFound,
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("encounter error when build request client or request: {0}")]
    WreqError(#[from] wreq::Error),
    #[error("encounter error when parse url from other type: {0}")]
    UrlParseError(#[from] url::ParseError),
}

#[derive(Debug, Error)]
pub enum AnimeEpisodeError {
    #[error("encounter error when parsing anime episode: {0}")]
    WreqError(#[from] wreq::Error),
    #[error("encounter error when parsing anime episode's html: {0}")]
    ParseHtmlError(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    SnListError(#[from] SnListError),
    #[error(transparent)]
    CookieError(#[from] CookieError),
    #[error(transparent)]
    RequestError(#[from] RequestError),
    #[error(transparent)]
    AnimeEpisodeError(#[from] AnimeEpisodeError),
}
