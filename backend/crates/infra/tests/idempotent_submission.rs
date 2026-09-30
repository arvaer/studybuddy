//! Idempotent submission by request key (#10): a resent key with the same
//! payload replays the receipt; a conflicting payload is refused and the
//! original is untouched; concurrent duplicates record exactly once.

use app::dtos::attempt::RecordAttemptRequest;
use app::errors::AppError;
use app::services::attempt::AttemptService;
use domain::errors::DomainError;
use infra::repositories::attempt::PgAttemptRepository;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// An exact-match activity whose key is "policy".
async fn revision(pool: &PgPool, user_id: Uuid) -> Uuid {
    let activity: Uuid = sqlx::query_scalar("INSERT INTO activities (user_id, kind) VALUES ($1, 'recall') RETURNING id")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query_scalar(
        "INSERT INTO activity_revisions (activity_id, revision, prompt, answer_key)
         VALUES ($1, 1, 'Name the mapping from states to actions.', '\"policy\"'::jsonb) RETURNING id",
    )
    .bind(activity)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn req(key: &str, revision_id: Uuid, response: Value, assistance: Vec<Value>) -> RecordAttemptRequest {
    RecordAttemptRequest { request_key: key.into(), activity_revision_id: revision_id, response, assistance }
}

async fn attempts(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM attempts").fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_key_and_payload_replays_the_receipt(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let first = svc.record(u, req("k", rev, json!("Policy"), vec![json!("hint")])).await.unwrap();
    assert!(!first.replayed);

    let again = svc.record(u, req("k", rev, json!("Policy"), vec![json!("hint")])).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.receipt.attempt_id, first.receipt.attempt_id);
    assert_eq!(again.receipt.status, "correct");
    assert_eq!(again.receipt.submitted_at, first.receipt.submitted_at);
    assert_eq!(attempts(&pool).await, 1);

    // A different key with the same payload is a genuinely new attempt.
    let other = svc.record(u, req("k2", rev, json!("Policy"), vec![json!("hint")])).await.unwrap();
    assert!(!other.replayed);
    assert_ne!(other.receipt.attempt_id, first.receipt.attempt_id);
    assert_eq!(attempts(&pool).await, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_key_with_a_different_payload_is_a_conflict_and_changes_nothing(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u).await;
    let rev2 = revision(&pool, u).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let first = svc.record(u, req("k", rev, json!("Policy"), vec![])).await.unwrap().receipt;

    // Different response, different assistance, different revision: each is refused.
    for bad in [
        req("k", rev, json!("Value function"), vec![]),
        req("k", rev, json!("Policy"), vec![json!("hint")]),
        req("k", rev2, json!("Policy"), vec![]),
    ] {
        let res = svc.record(u, bad).await;
        assert!(matches!(res, Err(AppError::Domain(DomainError::Conflict(_)))), "{res:?}");
    }

    // The original is untouched and still the only attempt.
    assert_eq!(attempts(&pool).await, 1);
    let stored: (Value, Value) =
        sqlx::query_as("SELECT response, assistance FROM attempts WHERE id = $1")
            .bind(first.attempt_id.parse::<Uuid>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, (json!("Policy"), json!([])));
    assert_eq!(svc.get(u, first.attempt_id.parse().unwrap()).await.unwrap().status, "correct");
}

#[sqlx::test(migrations = "../../migrations")]
async fn request_keys_are_scoped_per_learner(pool: PgPool) {
    let a = learner(&pool, "a@example.com").await;
    let b = learner(&pool, "b@example.com").await;
    let rev_a = revision(&pool, a).await;
    let rev_b = revision(&pool, b).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let mine = svc.record(a, req("shared", rev_a, json!("Policy"), vec![])).await.unwrap();
    let theirs = svc.record(b, req("shared", rev_b, json!("Policy"), vec![])).await.unwrap();
    assert!(!mine.replayed && !theirs.replayed);
    assert_ne!(mine.receipt.attempt_id, theirs.receipt.attempt_id);

    // B reusing A's key against A's revision is still ownership-refused,
    // not a conflict, and never leaks A's receipt.
    let res = svc.record(b, req("shared-2", rev_a, json!("Policy"), vec![])).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::NotFound(_)))));
    assert_eq!(attempts(&pool).await, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn blank_or_oversized_key_is_rejected_before_any_write(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    for key in ["", "   ", &"x".repeat(129)] {
        let res = svc.record(u, req(key, rev, json!("Policy"), vec![])).await;
        assert!(matches!(res, Err(AppError::Validation(_))), "{res:?}");
    }
    assert_eq!(attempts(&pool).await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_duplicates_record_exactly_once(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u).await;

    // Twelve identical submissions launched together, each on its own
    // connection. Exactly one wins the insert; the rest replay it.
    let tasks: Vec<_> = (0..12)
        .map(|_| {
            let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));
            tokio::spawn(async move { svc.record(u, req("race", rev, json!("Policy"), vec![])).await })
        })
        .collect();

    let mut ids = Vec::new();
    let mut fresh = 0;
    for t in tasks {
        let r = t.await.unwrap().unwrap();
        if !r.replayed {
            fresh += 1;
        }
        ids.push(r.receipt.attempt_id);
    }
    ids.dedup();

    assert_eq!(fresh, 1);
    assert_eq!(ids.len(), 1);
    assert_eq!(attempts(&pool).await, 1);
}
