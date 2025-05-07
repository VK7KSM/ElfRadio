use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use tracing::{error, warn};
use uuid;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Internal Server Error: {0}")]
    InternalServerError(String),

    #[error("Task with ID {0} not found")]
    TaskNotFound(uuid::Uuid),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ZIP archive creation error: {0}")]
    ZipError(#[from] zip::result::ZipError),

    #[error("Client channel send error: {0}")]
    ClientSendError(String),

    #[error("JSON serialization/deserialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Task is already running")]
    TaskAlreadyRunning,

    #[error("AI provider is not configured. Please configure in settings.")]
    AiNotConfigured,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad Gateway: Upstream API error (Status {0}) - {1}")]
    BadGateway(u16, String),

    #[error("Service Unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::BadRequest(msg) => {
                error!("Bad Request: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            ApiError::InternalServerError(msg) => {
                 error!("Internal Server Error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("An internal error occurred: {}", msg))
            }
            ApiError::TaskNotFound(id) => (StatusCode::NOT_FOUND, format!("Task with ID {} not found", id)),
            ApiError::IoError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("I/O Error: {}", e)),
            ApiError::ZipError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("ZIP Error: {}", e)),
            ApiError::ClientSendError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::JsonError(e) => (StatusCode::BAD_REQUEST, format!("JSON Error: {}", e)),
            ApiError::TaskAlreadyRunning => (StatusCode::CONFLICT, self.to_string()),
            ApiError::AiNotConfigured => (StatusCode::CONFLICT, self.to_string()),
            ApiError::Unauthorized(msg) => {
                warn!("API Unauthorized: {}", msg);
                (StatusCode::UNAUTHORIZED, msg)
            }
            ApiError::BadGateway(upstream_status, msg) => {
                error!("API Bad Gateway (Upstream Status: {}): {}", upstream_status, msg);
                (StatusCode::BAD_GATEWAY, format!("Upstream service error (Status {}): {}", upstream_status, msg))
            }
            ApiError::ServiceUnavailable(msg) => {
                error!("API Service Unavailable: {}", msg);
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
} 