use domain::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation: {0}")]
    Validation(String),

    #[error("Token error: {0}")]
    Token(String),

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

