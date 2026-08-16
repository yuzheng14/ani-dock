use ani_dock_core::{AnimeResolveError, ConfigError, CookieError};
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod router;
pub mod service;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("操作数据库发生错误：{0}")]
    Db(#[from] sqlx::Error),
    #[error("解析动画发生错误：{0}")]
    ResolveAnimeError(#[from] AnimeResolveError),
    #[error("未找到当前剧集，可能是未解析动画，剧集 sn 为 {0}")]
    EpisodeNotFound(u32),
    #[error("SSE 事件数据转换出错：{0}")]
    SSEEventJsonDataConvert(axum::Error),
    #[error("写入配置文件错误：{0}")]
    WriteConfig(#[from] ConfigError),
    #[error("写入 cookie 文件错误：{0}")]
    WriteCookie(#[from] CookieError),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    DbError,
    ResolveAnimeError,
    EpisodeNotFound,
    SSEEventJsonDataConvert,
    WriteConfig,
    WriteCookie,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    code: ErrorCode,
    message: String,
}

pub type ApiResult<T = ()> = Result<T, ApiError>;
pub type CoreEpisode = ani_dock_core::Episode;
pub type CoreAnime = ani_dock_core::Anime;

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = %self, "请求错误");
        match self {
            Self::Db(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: ErrorCode::DbError,
                    message: self.to_string(),
                }),
            )
                .into_response(),
            Self::ResolveAnimeError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: ErrorCode::ResolveAnimeError,
                    message: self.to_string(),
                }),
            )
                .into_response(),
            Self::EpisodeNotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    code: ErrorCode::EpisodeNotFound,
                    message: self.to_string(),
                }),
            )
                .into_response(),
            Self::SSEEventJsonDataConvert(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: ErrorCode::SSEEventJsonDataConvert,
                    message: self.to_string(),
                }),
            )
                .into_response(),
            Self::WriteConfig(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: ErrorCode::WriteConfig,
                    message: self.to_string(),
                }),
            )
                .into_response(),
            Self::WriteCookie(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    code: ErrorCode::WriteCookie,
                    message: self.to_string(),
                }),
            )
                .into_response(),
        }
    }
}
