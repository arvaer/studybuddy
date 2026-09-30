use serde::Serialize;

use domain::artifacts::Artifact;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactResponse {
    pub id:                String,
    pub sha256:            String,
    pub size_bytes:        i64,
    pub content_type:      String,
    pub original_filename: String,
    pub uploaded_at:       String,
}

impl From<Artifact> for ArtifactResponse {
    fn from(a: Artifact) -> Self {
        Self {
            id:                a.id.to_string(),
            sha256:            a.sha256,
            size_bytes:        a.size_bytes,
            content_type:      a.content_type,
            original_filename: a.original_filename,
            uploaded_at:       a.uploaded_at.to_rfc3339(),
        }
    }
}
