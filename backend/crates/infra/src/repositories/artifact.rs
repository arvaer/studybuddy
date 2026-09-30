//! Artifact metadata (#11). Bytes are the upload service's concern.

use sqlx::PgPool;
use uuid::Uuid;

use domain::artifacts::{Artifact, NewArtifact};
use domain::errors::DomainError;
use domain::repository_traits::ArtifactRepository;

#[derive(Clone)]
pub struct PgArtifactRepository {
    pool: PgPool,
}

impl PgArtifactRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db(e: sqlx::Error) -> DomainError {
    DomainError::Repository(e.to_string())
}

impl ArtifactRepository for PgArtifactRepository {
    async fn store(&self, cmd: NewArtifact) -> Result<Artifact, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO artifacts (user_id, sha256, size_bytes, content_type, original_filename)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, uploaded_at
            "#,
            cmd.user_id,
            cmd.sha256,
            cmd.size_bytes,
            cmd.content_type,
            cmd.original_filename,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;

        Ok(Artifact {
            id:                row.id,
            user_id:           cmd.user_id,
            sha256:            cmd.sha256,
            size_bytes:        cmd.size_bytes,
            content_type:      cmd.content_type,
            original_filename: cmd.original_filename,
            uploaded_at:       row.uploaded_at,
        })
    }

    async fn find(&self, id: Uuid, user_id: Uuid) -> Result<Artifact, DomainError> {
        sqlx::query_as!(
            Artifact,
            r#"
            SELECT id, user_id, sha256, size_bytes, content_type, original_filename, uploaded_at
            FROM artifacts WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound(format!("artifact {id}")))
    }

    async fn addresses(&self) -> Result<Vec<String>, DomainError> {
        sqlx::query_scalar!("SELECT DISTINCT sha256 FROM artifacts ORDER BY sha256")
            .fetch_all(&self.pool)
            .await
            .map_err(db)
    }
}
