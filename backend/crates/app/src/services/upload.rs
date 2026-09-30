use std::path::PathBuf;
use uuid::Uuid;

use domain::repository_traits::{ArtifactRepository, ResourceRepository};

use crate::dtos::resource::ResourceResponse;
use crate::errors::AppError;
use crate::services::artifact::ArtifactStore;

pub struct UploadService<R: ResourceRepository, A: ArtifactRepository> {
    repo:      R,
    artifacts: ArtifactStore<A>,
}

impl<R: ResourceRepository, A: ArtifactRepository> UploadService<R, A> {
    pub fn new(repo: R, artifacts: A, uploads_dir: PathBuf) -> Self {
        Self { repo, artifacts: ArtifactStore::new(artifacts, uploads_dir) }
    }

    /// Store the bytes as an immutable artifact, then create the Resource
    /// that points at it (#11). The bytes are addressed by content, never by
    /// filename, so a later upload with the same name is a new artifact and
    /// cannot replace this one.
    ///
    /// `content_text`/`content_pages` are pre-extracted by the caller.
    #[allow(clippy::too_many_arguments)]
    pub async fn upload(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        title: String,
        filename: String,
        content_type: &str,
        bytes: &[u8],
        content_text: String,
        content_pages: Vec<String>,
        resource_type: &str,
        concept_ids: Vec<Uuid>,
    ) -> Result<ResourceResponse, AppError> {
        let artifact = self.artifacts.put(user_id, &filename, content_type, bytes).await?;
        let file_path = self.artifacts.blob_path(&artifact.sha256).to_string_lossy().to_string();

        let resource = self
            .repo
            .create_uploaded(
                user_id,
                topic_id,
                &title,
                resource_type,
                artifact.id,
                &file_path,
                &content_text,
                &content_pages,
                &concept_ids,
            )
            .await?;

        Ok(ResourceResponse::from(resource))
    }

    pub async fn get_content(&self, id: Uuid, user_id: Uuid) -> Result<String, AppError> {
        let text = self.repo.get_content_text(id, user_id).await?.unwrap_or_default();
        Ok(text)
    }
}
