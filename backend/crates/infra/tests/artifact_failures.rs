//! Failure behaviour of the artifact store (#12): a blob written before its
//! row fails, a row whose blob is gone, an interrupted write, and the audit
//! that reports each without deleting anything.

use std::path::PathBuf;

use app::errors::AppError;
use app::services::artifact::{content_address, ArtifactAudit, ArtifactStore};
use domain::artifacts::{Artifact, NewArtifact};
use domain::errors::DomainError;
use domain::repository_traits::ArtifactRepository;
use infra::repositories::artifact::PgArtifactRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
        .bind(format!("{}@example.com", Uuid::new_v4()))
        .fetch_one(pool)
        .await
        .unwrap()
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("studybuddy-artifact-failures-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn store(pool: &PgPool, dir: &PathBuf) -> ArtifactStore<PgArtifactRepository> {
    ArtifactStore::new(PgArtifactRepository::new(pool.clone()), dir.clone())
}

/// A catalog whose insert always fails: the database went away between the
/// blob write and the row write.
struct RowInsertFails(PgArtifactRepository);

impl ArtifactRepository for RowInsertFails {
    async fn store(&self, _: NewArtifact) -> Result<Artifact, DomainError> {
        Err(DomainError::Repository("connection reset".into()))
    }
    async fn find(&self, id: Uuid, user_id: Uuid) -> Result<Artifact, DomainError> {
        self.0.find(id, user_id).await
    }
    async fn addresses(&self) -> Result<Vec<String>, DomainError> {
        self.0.addresses().await
    }
}

fn unexpected_containing<T: std::fmt::Debug>(res: Result<T, AppError>, needle: &str) {
    match res {
        Err(AppError::Unexpected(msg)) if msg.contains(needle) => {}
        other => panic!("expected Unexpected containing {needle:?}, got {other:?}"),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn blob_written_but_row_fails_leaves_a_reusable_orphan_and_no_row(pool: PgPool) {
    let dir = scratch();
    let u = learner(&pool).await;
    let bytes = b"the bytes of an upload whose row never landed";
    let address = content_address(bytes);

    let broken = ArtifactStore::new(RowInsertFails(PgArtifactRepository::new(pool.clone())), dir.clone());
    let res = broken.put(u, "notes.txt", "text/plain", bytes).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::Repository(_)))), "{res:?}");

    // The blob is on disk, complete, at its address; the catalog has no row.
    let good = store(&pool, &dir);
    assert_eq!(std::fs::read(good.blob_path(&address)).unwrap(), bytes);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM artifacts").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 0);
    assert_eq!(good.audit().await.unwrap(), ArtifactAudit { orphans: vec![address.clone()], ..Default::default() });

    // Retrying the upload reuses the orphan and lands the row; the audit is clean.
    let artifact = good.put(u, "notes.txt", "text/plain", bytes).await.unwrap();
    assert_eq!(artifact.sha256, address);
    assert_eq!(good.read(u, artifact.id).await.unwrap().1, bytes);
    assert!(good.audit().await.unwrap().is_clean());

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn row_without_blob_is_refused_on_read_and_reported_as_missing(pool: PgPool) {
    let dir = scratch();
    let u = learner(&pool).await;
    let store = store(&pool, &dir);
    let artifact = store.put(u, "notes.txt", "text/plain", b"bytes that will be lost").await.unwrap();

    // The blob directory loses the file (a disk restored from an older backup).
    std::fs::remove_file(store.blob_path(&artifact.sha256)).unwrap();

    // Metadata still answers; bytes are refused with the address, not 404.
    assert_eq!(store.get(u, artifact.id).await.unwrap().sha256, artifact.sha256);
    unexpected_containing(store.read(u, artifact.id).await, &format!("blob missing at {}", artifact.sha256));
    assert_eq!(store.audit().await.unwrap(), ArtifactAudit { missing: vec![artifact.sha256.clone()], ..Default::default() });

    // Restoring the exact bytes at the address heals it: nothing else to fix.
    std::fs::write(store.blob_path(&artifact.sha256), b"bytes that will be lost").unwrap();
    assert_eq!(store.read(u, artifact.id).await.unwrap().1, b"bytes that will be lost");
    assert!(store.audit().await.unwrap().is_clean());

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn interrupted_write_leaves_a_part_file_that_is_never_a_blob(pool: PgPool) {
    let dir = scratch();
    let u = learner(&pool).await;
    let store = store(&pool, &dir);
    let artifact = store.put(u, "notes.txt", "text/plain", b"complete").await.unwrap();

    // A writer that died before its rename: the temp file next to the blob.
    let shard = store.blob_path(&artifact.sha256).parent().unwrap().to_path_buf();
    let part = shard.join(format!(".{}.part", Uuid::new_v4()));
    std::fs::write(&part, b"half of some upl").unwrap();
    // And a file that is neither a blob nor a part.
    std::fs::write(shard.join("README"), b"ignored").unwrap();

    let audit = store.audit().await.unwrap();
    assert_eq!(audit, ArtifactAudit { parts: vec![part.clone()], ..Default::default() });
    assert!(!audit.is_clean());

    // A successful put leaves no part of its own behind.
    let entries: Vec<_> = std::fs::read_dir(&shard).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
    assert_eq!(entries.iter().filter(|n| n.ends_with(".part")).count(), 1);

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_puts_of_the_same_bytes_leave_one_blob_and_no_parts(pool: PgPool) {
    let dir = scratch();
    let u = learner(&pool).await;
    let bytes = b"raced";
    let store = std::sync::Arc::new(store(&pool, &dir));

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let store = store.clone();
            tokio::spawn(async move { store.put(u, "notes.txt", "text/plain", bytes).await })
        })
        .collect();
    let mut ids = std::collections::HashSet::new();
    for h in handles {
        ids.insert(h.await.unwrap().unwrap().id);
    }
    assert_eq!(ids.len(), 8);

    let audit = store.audit().await.unwrap();
    assert!(audit.is_clean(), "{audit:?}");
    let shard = store.blob_path(&content_address(bytes)).parent().unwrap().to_path_buf();
    assert_eq!(std::fs::read_dir(&shard).unwrap().count(), 1);

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_of_an_empty_store_is_clean(pool: PgPool) {
    let dir = scratch();
    assert!(store(&pool, &dir).audit().await.unwrap().is_clean());
    std::fs::remove_dir_all(dir).unwrap();
}
