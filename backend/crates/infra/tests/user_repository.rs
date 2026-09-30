//! Repository tests run against a real PostgreSQL server.
//!
//! `#[sqlx::test]` creates a fresh, uniquely named database on the server at
//! `DATABASE_URL` for every test, applies `backend/migrations`, and drops the
//! database afterwards. Point `DATABASE_URL` at the disposable container from
//! `scripts/dev-db.sh`; never at a database you care about.

use domain::errors::DomainError;
use domain::repository_traits::UserRepository;
use infra::repositories::user::PgUserRepository;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn create_then_find_by_email_round_trips(pool: PgPool) {
    let repo = PgUserRepository::new(pool);

    let created = repo
        .create("ada@example.test", "correct horse", "Ada")
        .await
        .expect("create user");

    let found = repo
        .find_by_email("ada@example.test")
        .await
        .expect("find by email");

    assert_eq!(found.id, created.id);
    assert_eq!(found.display_name, "Ada");
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_email_is_a_conflict(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    repo.create("ada@example.test", "pw", "Ada").await.expect("first create");

    let second = repo.create("ada@example.test", "pw", "Ada again").await;

    assert!(matches!(second, Err(DomainError::Conflict(_))), "got {second:?}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn wrong_password_is_unauthorized(pool: PgPool) {
    let repo = PgUserRepository::new(pool);
    repo.create("ada@example.test", "correct horse", "Ada").await.expect("create");

    let ok = repo.verify_password("ada@example.test", "correct horse").await;
    let bad = repo.verify_password("ada@example.test", "battery staple").await;

    assert!(ok.is_ok(), "got {ok:?}");
    assert!(matches!(bad, Err(DomainError::Unauthorized)), "got {bad:?}");
}
