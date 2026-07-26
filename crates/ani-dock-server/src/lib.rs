use ani_dock_core::AnimeResolveError;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod router;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("操作数据库发生错误：{0}")]
    Db(#[from] sqlx::Error),
    #[error("解析动画发生错误：{0}")]
    ResolveAnimeError(#[from] AnimeResolveError),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    DbError,
    ResolveAnimeError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    code: ErrorCode,
    message: String,
}

pub type ApiResult<T> = Result<T, ApiError>;

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
        }
    }
}
