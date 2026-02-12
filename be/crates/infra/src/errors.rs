use domain::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Pool error: {0}")]
    Pool(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Not found")]
    NotFound,
}

impl From<InfraError> for DomainError {
    fn from(err: InfraError) -> Self {
        match err {
            InfraError::NotFound => DomainError::NotFound("record not found".into()),
            other => DomainError::Repository(other.to_string()),
        }
    }
}
