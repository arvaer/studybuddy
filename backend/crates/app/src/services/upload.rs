use std::path::PathBuf;
use uuid::Uuid;

use domain::repository_traits::ResourceRepository;

use crate::dtos::resource::ResourceResponse;
use crate::errors::AppError;

pub struct UploadService<R: ResourceRepository> {
    repo:        R,
    uploads_dir: PathBuf,
}

impl<R: ResourceRepository> UploadService<R> {
    pub fn new(repo: R, uploads_dir: PathBuf) -> Self {
        Self { repo, uploads_dir }
    }

    /// Persist an uploaded file to disk and create a Resource record.
    ///
    /// `content_text` should be pre-extracted by the caller (e.g. PDF text
    /// extraction happens in the route handler which has access to `infra`).
    pub async fn upload(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        title: String,
        filename: String,
        bytes: &[u8],
        content_text: String,
        content_pages: Vec<String>,
        resource_type: &str,
        concept_ids: Vec<Uuid>,
    ) -> Result<ResourceResponse, AppError> {
        // Persist file to disk
        let user_dir = self.uploads_dir.join(user_id.to_string());
        tokio::fs::create_dir_all(&user_dir)
            .await
            .map_err(|e| AppError::Unexpected(format!("create upload dir: {e}")))?;

        let safe_name = sanitize_filename(&filename);
        let file_path = user_dir.join(&safe_name);
        tokio::fs::write(&file_path, bytes)
            .await
            .map_err(|e| AppError::Unexpected(format!("write upload file: {e}")))?;

        let file_path_str = file_path.to_string_lossy().to_string();

        let resource = self
            .repo
            .create_uploaded(
                user_id,
                topic_id,
                &title,
                resource_type,
                &file_path_str,
                &content_text,
                &content_pages,
                &concept_ids,
            )
            .await?;

        Ok(ResourceResponse::from(resource))
    }

    pub async fn get_content(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<String, AppError> {
        let text = self
            .repo
            .get_content_text(id, user_id)
            .await?
            .unwrap_or_default();
        Ok(text)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
