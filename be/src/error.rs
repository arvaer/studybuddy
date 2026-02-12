use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use app::errors::AppError;
use domain::errors::DomainError;

/// Central HTTP error type. Wraps AppError into an axum Response.
pub struct HttpError(pub AppError);

impl From<AppError> for HttpError {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl From<DomainError> for HttpError {
    fn from(e: DomainError) -> Self {
        Self(AppError::Domain(e))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            AppError::Domain(DomainError::NotFound(msg)) => {
                (StatusCode::NOT_FOUND, msg.clone())
            }
            AppError::Domain(DomainError::Validation(msg)) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
            }
            AppError::Domain(DomainError::Unauthorized) => {
                (StatusCode::UNAUTHORIZED, "unauthorized".to_string())
            }
            AppError::Domain(DomainError::Conflict(msg)) => {
                (StatusCode::CONFLICT, msg.clone())
            }
            AppError::Domain(DomainError::Repository(msg)) => {
                tracing::error!("repository error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".to_string())
            }
            AppError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            AppError::Conflict(msg) => {
                (StatusCode::CONFLICT, msg.clone())
            }
            AppError::Validation(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone())
            }
            AppError::Token(msg) => {
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            AppError::Unexpected(msg) => {
                tracing::error!("unexpected error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
