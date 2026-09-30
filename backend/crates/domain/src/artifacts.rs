//! Artifacts (#11): immutable, content-addressed records of uploaded bytes.
//! The address is the SHA-256 of the bytes; two uploads of identical bytes
//! share one blob on disk but are two artifacts, each with its own metadata.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Artifact {
    pub id:                Uuid,
    pub user_id:           Uuid,
    /// Lower-case hex SHA-256 of the bytes: the content address.
    pub sha256:            String,
    pub size_bytes:        i64,
    pub content_type:      String,
    pub original_filename: String,
    pub uploaded_at:       DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewArtifact {
    pub user_id:           Uuid,
    pub sha256:            String,
    pub size_bytes:        i64,
    pub content_type:      String,
    pub original_filename: String,
}

/// Where the bytes for `sha256` live, relative to the uploads directory.
/// Sharded by the first two hex digits so no directory grows unbounded.
pub fn blob_relative_path(sha256: &str) -> String {
    format!("artifacts/{}/{}", &sha256[..2], sha256)
}
