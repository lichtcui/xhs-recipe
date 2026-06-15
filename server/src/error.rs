use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Error codes for the server API.
///
/// Used for structured error responses. Currently reserved for future
/// endpoint error handling (errors are inline in SSE for /process).
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("INVALID_URL: {0}")]
    InvalidUrl(String),
    #[error("FETCH_FAILED: {0}")]
    FetchFailed(String),
    #[error("UNSUPPORTED_CONTENT: {0}")]
    UnsupportedContent(String),
    #[error("DOWNLOAD_FAILED: {0}")]
    DownloadFailed(String),
    #[error("PROCESSING_FAILED: {0}")]
    ProcessingFailed(String),
    #[error("TIMEOUT")]
    Timeout,
    #[error("INTERNAL_ERROR: {0}")]
    Internal(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (code, status, message) = match &self {
            ServerError::InvalidUrl(msg) => ("INVALID_URL", StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::FetchFailed(msg) => ("FETCH_FAILED", StatusCode::BAD_GATEWAY, msg.clone()),
            ServerError::UnsupportedContent(msg) => ("UNSUPPORTED_CONTENT", StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            ServerError::DownloadFailed(msg) => ("DOWNLOAD_FAILED", StatusCode::BAD_GATEWAY, msg.clone()),
            ServerError::ProcessingFailed(msg) => ("PROCESSING_FAILED", StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ServerError::Timeout => ("TIMEOUT", StatusCode::GATEWAY_TIMEOUT, "processing timed out".into()),
            ServerError::Internal(msg) => ("INTERNAL_ERROR", StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = json!({ "code": code, "message": message });
        (status, Json(body)).into_response()
    }
}
