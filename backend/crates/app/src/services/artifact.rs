//! The artifact store (#11): content-addressed blobs under the uploads
//! directory plus an immutable metadata row per upload.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use domain::artifacts::{blob_relative_path, Artifact, NewArtifact};
use domain::repository_traits::ArtifactRepository;

use crate::dtos::artifact::ArtifactResponse;
use crate::errors::AppError;

pub struct ArtifactStore<R: ArtifactRepository> {
    repo:        R,
    uploads_dir: PathBuf,
}

pub fn content_address(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

impl<R: ArtifactRepository> ArtifactStore<R> {
    pub fn new(repo: R, uploads_dir: PathBuf) -> Self {
        Self { repo, uploads_dir }
    }

    pub fn blob_path(&self, sha256: &str) -> PathBuf {
        self.uploads_dir.join(blob_relative_path(sha256))
    }

    /// Persist `bytes` for `user_id`. The blob is written to a temporary
    /// file and renamed into its content address, so a reader never sees a
    /// partial file and an existing blob is never overwritten: identical
    /// bytes already on disk are simply reused. The metadata row is written
    /// after the blob exists. Failure between the two leaves a blob with no
    /// row, which is harmless and is #12's orphan case.
    pub async fn put(
        &self,
        user_id: Uuid,
        original_filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<Artifact, AppError> {
        let sha256 = content_address(bytes);
        let path = self.blob_path(&sha256);
        if !tokio::fs::try_exists(&path).await.map_err(|e| AppError::Unexpected(format!("stat blob: {e}")))? {
            write_atomically(&path, bytes).await?;
        }

        let artifact = self
            .repo
            .store(NewArtifact {
                user_id,
                sha256,
                size_bytes: bytes.len() as i64,
                content_type: content_type.to_string(),
                original_filename: original_filename.to_string(),
            })
            .await?;
        Ok(artifact)
    }

    pub async fn get(&self, user_id: Uuid, id: Uuid) -> Result<ArtifactResponse, AppError> {
        Ok(self.repo.find(id, user_id).await?.into())
    }

    /// The metadata and the exact bytes, verified against the address.
    pub async fn read(&self, user_id: Uuid, id: Uuid) -> Result<(Artifact, Vec<u8>), AppError> {
        let artifact = self.repo.find(id, user_id).await?;
        let bytes = tokio::fs::read(self.blob_path(&artifact.sha256))
            .await
            .map_err(|e| AppError::Unexpected(format!("read blob: {e}")))?;
        if content_address(&bytes) != artifact.sha256 {
            return Err(AppError::Unexpected(format!("artifact {id} bytes do not match their address")));
        }
        Ok((artifact, bytes))
    }
}

async fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let dir = path.parent().ok_or_else(|| AppError::Unexpected("blob path has no parent".into()))?;
    tokio::fs::create_dir_all(dir).await.map_err(|e| AppError::Unexpected(format!("create blob dir: {e}")))?;
    let tmp = dir.join(format!(".{}.{}", Uuid::new_v4(), "part"));
    tokio::fs::write(&tmp, bytes).await.map_err(|e| AppError::Unexpected(format!("write blob: {e}")))?;
    if let Err(e) = tokio::fs::rename(&tmp, path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        // A concurrent writer of the same bytes may have won the rename.
        if tokio::fs::try_exists(path).await.unwrap_or(false) {
            return Ok(());
        }
        return Err(AppError::Unexpected(format!("commit blob: {e}")));
    }
    Ok(())
}
