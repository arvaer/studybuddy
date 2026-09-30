//! The artifact store (#11): uploads are addressed by content, a same-name
//! upload is a new version that cannot touch an earlier one, a revision pins
//! the version it cited, and reads are owner-scoped.

use std::path::PathBuf;

use app::dtos::activity::{CreateActivityRequest, RevisionContentRequest};
use app::errors::AppError;
use app::services::activity::ActivityService;
use app::services::artifact::{content_address, ArtifactStore};
use app::services::upload::UploadService;
use domain::errors::DomainError;
use infra::repositories::activity::PgActivityRepository;
use infra::repositories::artifact::PgArtifactRepository;
use infra::repositories::resource::PgResourceRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool, email: &str) -> (Uuid, Uuid) {
    let user: Uuid =
        sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
            .bind(email)
            .fetch_one(pool)
            .await
            .unwrap();
    let topic: Uuid = sqlx::query_scalar("INSERT INTO topics (user_id, name) VALUES ($1, 'RL') RETURNING id")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap();
    (user, topic)
}

/// A scratch uploads directory per test, removed at the end of the test.
fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("studybuddy-artifacts-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn uploads(pool: &PgPool, dir: &PathBuf) -> UploadService<PgResourceRepository, PgArtifactRepository> {
    UploadService::new(PgResourceRepository::new(pool.clone()), PgArtifactRepository::new(pool.clone()), dir.clone())
}

fn store(pool: &PgPool, dir: &PathBuf) -> ArtifactStore<PgArtifactRepository> {
    ArtifactStore::new(PgArtifactRepository::new(pool.clone()), dir.clone())
}

async fn upload(svc: &UploadService<PgResourceRepository, PgArtifactRepository>, user: Uuid, topic: Uuid, name: &str, bytes: &[u8]) -> (Uuid, Uuid) {
    let r = svc
        .upload(user, topic, "Notes".into(), name.into(), "text/plain", bytes, String::from_utf8_lossy(bytes).into(), vec![], "article", vec![])
        .await
        .unwrap();
    (r.id.parse().unwrap(), r.artifact_id.unwrap().parse().unwrap())
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_name_upload_is_a_new_version_and_the_earlier_one_is_untouched(pool: PgPool) {
    let dir = scratch();
    let (u, topic) = learner(&pool, "a@example.com").await;
    let svc = uploads(&pool, &dir);
    let store = store(&pool, &dir);

    let (res1, art1) = upload(&svc, u, topic, "notes.txt", b"version one").await;
    let (res2, art2) = upload(&svc, u, topic, "notes.txt", b"version two").await;
    assert_ne!(res1, res2);
    assert_ne!(art1, art2);

    // Each artifact reads back its own bytes; the first was not replaced.
    let (meta1, bytes1) = store.read(u, art1).await.unwrap();
    let (meta2, bytes2) = store.read(u, art2).await.unwrap();
    assert_eq!(bytes1, b"version one");
    assert_eq!(bytes2, b"version two");
    assert_eq!(meta1.original_filename, "notes.txt");
    assert_eq!(meta2.original_filename, "notes.txt");
    assert_ne!(meta1.sha256, meta2.sha256);
    assert_eq!(meta1.sha256, content_address(b"version one"));
    assert_eq!((meta1.size_bytes, meta1.content_type.as_str()), (11, "text/plain"));

    // Two blobs on disk, at their content addresses, not under the filename.
    assert!(store.blob_path(&meta1.sha256).is_file());
    assert!(store.blob_path(&meta2.sha256).is_file());
    assert!(!dir.join(u.to_string()).exists());

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_revision_pins_the_source_version_it_was_authored_against(pool: PgPool) {
    let dir = scratch();
    let (u, topic) = learner(&pool, "a@example.com").await;
    let svc = uploads(&pool, &dir);
    let activities = ActivityService::new(PgActivityRepository::new(pool.clone()));

    let (res1, art1) = upload(&svc, u, topic, "sutton.txt", b"chapter 3, first edition").await;
    let created = activities
        .create(u, CreateActivityRequest {
            kind: "recall".into(),
            concept_id: None,
            revision: RevisionContentRequest {
                prompt: "What is a policy?".into(),
                options: None,
                answer_key: None,
                rubric: None,
                source_resource_id: Some(res1),
                source_location: Some(serde_json::json!({"page": 3})),
            },
        })
        .await
        .unwrap();
    assert_eq!(created.current.source_artifact_id, Some(art1.to_string()));

    // A later same-name upload does not change what the revision cites,
    // and the cited bytes are still the first edition.
    let (_res2, art2) = upload(&svc, u, topic, "sutton.txt", b"chapter 3, second edition").await;
    assert_ne!(art1, art2);
    let rev = activities.get_revision(u, created.current.id.parse().unwrap()).await.unwrap();
    assert_eq!(rev.source_artifact_id, Some(art1.to_string()));
    let (_, bytes) = store(&pool, &dir).read(u, art1).await.unwrap();
    assert_eq!(bytes, b"chapter 3, first edition");

    // The cited artifact cannot be deleted out from under the revision.
    let del = sqlx::query("DELETE FROM artifacts WHERE id = $1").bind(art1).execute(&pool).await;
    assert!(del.is_err());

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn identical_bytes_share_one_blob_but_are_two_artifacts(pool: PgPool) {
    let dir = scratch();
    let (u, topic) = learner(&pool, "a@example.com").await;
    let svc = uploads(&pool, &dir);

    let (_, art1) = upload(&svc, u, topic, "a.txt", b"same bytes").await;
    let (_, art2) = upload(&svc, u, topic, "b.txt", b"same bytes").await;
    assert_ne!(art1, art2);

    let store = store(&pool, &dir);
    let (m1, _) = store.read(u, art1).await.unwrap();
    let (m2, _) = store.read(u, art2).await.unwrap();
    assert_eq!(m1.sha256, m2.sha256);
    assert_eq!((m1.original_filename.as_str(), m2.original_filename.as_str()), ("a.txt", "b.txt"));
    let blobs = walkdir_count(&dir.join("artifacts"));
    assert_eq!(blobs, 1);

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn artifacts_are_owner_scoped_and_immutable(pool: PgPool) {
    let dir = scratch();
    let (a, topic_a) = learner(&pool, "a@example.com").await;
    let (b, _) = learner(&pool, "b@example.com").await;
    let svc = uploads(&pool, &dir);
    let store = store(&pool, &dir);

    let (_, art) = upload(&svc, a, topic_a, "private.txt", b"mine").await;

    let peek = store.get(b, art).await;
    assert!(matches!(peek, Err(AppError::Domain(DomainError::NotFound(_)))), "{peek:?}");
    let peek = store.read(b, art).await;
    assert!(matches!(peek, Err(AppError::Domain(DomainError::NotFound(_)))), "{peek:?}");
    assert!(store.get(a, art).await.is_ok());

    let upd = sqlx::query("UPDATE artifacts SET original_filename = 'renamed' WHERE id = $1").bind(art).execute(&pool).await;
    assert!(upd.is_err());

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn corrupted_blob_is_refused_rather_than_served(pool: PgPool) {
    let dir = scratch();
    let (u, topic) = learner(&pool, "a@example.com").await;
    let svc = uploads(&pool, &dir);
    let store = store(&pool, &dir);

    let (_, art) = upload(&svc, u, topic, "x.txt", b"original").await;
    let (meta, _) = store.read(u, art).await.unwrap();
    std::fs::write(store.blob_path(&meta.sha256), b"tampered").unwrap();

    let res = store.read(u, art).await;
    assert!(matches!(res, Err(AppError::Unexpected(_))), "{res:?}");

    std::fs::remove_dir_all(dir).unwrap();
}

fn walkdir_count(dir: &PathBuf) -> usize {
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            n += walkdir_count(&entry.path());
        } else {
            n += 1;
        }
    }
    n
}
