use ani_dock_db::model::CoverImage;
use axum::{
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    response::{IntoResponse, Response},
};

use crate::ApiError;

pub(super) const CACHE_CONTROL_VALUE: &str = "public, max-age=604800";
pub(super) const NOT_FOUND_CACHE_CONTROL_VALUE: &str = "public, max-age=300";

pub(super) fn response(request_headers: &HeaderMap, cover_image: CoverImage) -> Response {
    let CoverImage {
        id,
        mime_type,
        bytes,
        ..
    } = cover_image;
    // Cover image rows are immutable, so the row ID is a strong representation validator.
    let etag = format!("\"{id}\"");
    let mut response = if if_none_match(request_headers, &etag) {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        ([(CONTENT_TYPE, mime_type)], bytes).into_response()
    };

    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(CACHE_CONTROL_VALUE));
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&etag).expect("UUID-based ETag should be a valid header value"),
    );

    response
}

pub(super) fn not_found() -> Response {
    let mut response = ApiError::NotFound.into_response();
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static(NOT_FOUND_CACHE_CONTROL_VALUE),
    );
    response
}

fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|candidate| {
            candidate == "*" || strip_weak_prefix(candidate) == strip_weak_prefix(etag)
        })
}

fn strip_weak_prefix(etag: &str) -> &str {
    etag.strip_prefix("W/").unwrap_or(etag)
}

#[cfg(test)]
mod tests {
    use axum::http::header::IF_NONE_MATCH;

    use super::*;

    #[test]
    fn if_none_match_supports_lists_and_weak_comparison() {
        let mut headers = HeaderMap::new();
        headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_static("\"other\", W/\"cover-id\""),
        );

        assert!(if_none_match(&headers, "\"cover-id\""));
        assert!(!if_none_match(&headers, "\"missing\""));
    }

    #[test]
    fn if_none_match_supports_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("*"));

        assert!(if_none_match(&headers, "\"cover-id\""));
    }
}
