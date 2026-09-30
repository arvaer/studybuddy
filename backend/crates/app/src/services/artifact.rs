//! The artifact store (#11): content-addressed blobs under the uploads
//! directory plus an immutable metadata row per upload.
//!
//! Failure behaviour (#12). The blob is written before the row, so the two
//! stores can only disagree in one direction: a blob may exist with no row
//! (an orphan, harmless and reused by the next upload of the same bytes),
//! but a row is never written before its bytes are durable on disk. A row
//! whose blob has since gone missing is data loss; `read` refuses it with a
//! distinct message and `audit` lists it. Nothing here deletes anything.

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
    /// row: an orphan, which the next `put` of the same bytes simply reuses
    /// and which `audit` reports. A failure inside `write_atomically` leaves
    /// at most a `.<uuid>.part` file, never a half-written blob at an address.
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

    /// The metadata and the exact bytes, verified against the address. A row
    /// whose blob is missing or altered is refused as an unexpected error
    /// (the catalog says it exists, so it is not `NotFound`); the message
    /// names the address so the operator can restore it from backup.
    pub async fn read(&self, user_id: Uuid, id: Uuid) -> Result<(Artifact, Vec<u8>), AppError> {
        let artifact = self.repo.find(id, user_id).await?;
        let bytes = match tokio::fs::read(self.blob_path(&artifact.sha256)).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::Unexpected(format!("artifact {id} blob missing at {}", artifact.sha256)));
            }
            Err(e) => return Err(AppError::Unexpected(format!("read blob: {e}"))),
        };
        if content_address(&bytes) != artifact.sha256 {
            return Err(AppError::Unexpected(format!("artifact {id} bytes do not match their address {}", artifact.sha256)));
        }
        Ok((artifact, bytes))
    }

    /// Compare the catalog with the blob directory. Reports only; the plan
    /// authorises no deletion of uploaded documents, so cleanup of orphans
    /// and stray parts is an operator's decision made from this report.
    pub async fn audit(&self) -> Result<ArtifactAudit, AppError> {
        let known: std::collections::BTreeSet<String> = self.repo.addresses().await?.into_iter().collect();
        let on_disk = list_blobs(&self.uploads_dir.join("artifacts")).await?;

        let missing = known.iter().filter(|a| !on_disk.blobs.contains(*a)).cloned().collect();
        let orphans = on_disk.blobs.iter().filter(|a| !known.contains(*a)).cloned().collect();
        Ok(ArtifactAudit { missing, orphans, parts: on_disk.parts })
    }
}

/// The result of `ArtifactStore::audit`.
#[derive(Debug, Default, PartialEq)]
pub struct ArtifactAudit {
    /// Addresses the catalog has a row for but the directory has no blob for.
    /// Data loss until restored; every read of such an artifact fails.
    pub missing: Vec<String>,
    /// Blobs no row refers to: a `put` that failed after writing, or a row
    /// removed with its user. Harmless; the bytes are reused if re-uploaded.
    pub orphans: Vec<String>,
    /// `.<uuid>.part` files: a write interrupted before its rename. Never
    /// served, never mistaken for a blob.
    pub parts:   Vec<PathBuf>,
}

impl ArtifactAudit {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.orphans.is_empty() && self.parts.is_empty()
    }
}

#[derive(Default)]
struct OnDisk {
    blobs: std::collections::BTreeSet<String>,
    parts: Vec<PathBuf>,
}

/// Walk `<uploads>/artifacts/<xx>/`. A file is a blob when its name is a
/// 64-hex address in the shard its address says; a `.part` file is a stray
/// temporary; anything else is ignored.
async fn list_blobs(root: &Path) -> Result<OnDisk, AppError> {
    let io = |e: std::io::Error| AppError::Unexpected(format!("audit blob dir: {e}"));
    let mut out = OnDisk::default();
    let mut shards = match tokio::fs::read_dir(root).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(io(e)),
    };
    while let Some(shard) = shards.next_entry().await.map_err(io)? {
        if !shard.file_type().await.map_err(io)?.is_dir() {
            continue;
        }
        let shard_name = shard.file_name().to_string_lossy().to_string();
        let mut files = tokio::fs::read_dir(shard.path()).await.map_err(io)?;
        while let Some(file) = files.next_entry().await.map_err(io)? {
            let name = file.file_name().to_string_lossy().to_string();
            if name.ends_with(".part") {
                out.parts.push(file.path());
            } else if is_address(&name) && name.starts_with(&shard_name) {
                out.blobs.insert(name);
            }
        }
    }
    Ok(out)
}

fn is_address(name: &str) -> bool {
    name.len() == 64 && name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
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
