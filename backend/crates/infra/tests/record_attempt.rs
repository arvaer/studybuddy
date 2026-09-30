//! The attempt-recording operation (#9): success with deterministic
//! assessment, explicit pending, ownership, and all-or-nothing writes.

use app::dtos::attempt::RecordAttemptRequest;
use app::errors::AppError;
use app::services::attempt::AttemptService;
use domain::errors::DomainError;
use domain::learning::{AssessmentMethod, AssessmentOutcome};
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

/// One activity with one revision. `options`/`answer_key` shape the rule.
async fn revision(pool: &PgPool, user_id: Uuid, options: Option<Value>, answer_key: Option<Value>) -> Uuid {
    let activity: Uuid = sqlx::query_scalar("INSERT INTO activities (user_id, kind) VALUES ($1, 'recall') RETURNING id")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query_scalar(
        "INSERT INTO activity_revisions (activity_id, revision, prompt, options, answer_key)
         VALUES ($1, 1, 'What does a policy specify?', $2, $3) RETURNING id",
    )
    .bind(activity)
    .bind(options)
    .bind(answer_key)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn req(revision_id: Uuid, response: Value) -> RecordAttemptRequest {
    RecordAttemptRequest { activity_revision_id: revision_id, response, assistance: vec![] }
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn keyed_activity_is_assessed_in_the_same_operation(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u, None, Some(json!("a mapping from states to actions"))).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let receipt = svc.record(u, req(rev, json!("A mapping from states to actions"))).await.unwrap();

    assert_eq!(receipt.status, "correct");
    let a = receipt.assessment.unwrap();
    assert_eq!((a.outcome, a.method, a.score), (AssessmentOutcome::Correct, AssessmentMethod::ExactMatch, Some(1.0)));
    assert_eq!(count(&pool, "attempts").await, 1);
    assert_eq!(count(&pool, "assessments").await, 1);

    // Reading it back agrees with the receipt.
    let again = svc.get(u, receipt.attempt_id.parse().unwrap()).await.unwrap();
    assert_eq!(again.status, "correct");
    assert_eq!(again.assessment.unwrap().method, AssessmentMethod::ExactMatch);
}

#[sqlx::test(migrations = "../../migrations")]
async fn choice_activity_wrong_option_is_incorrect(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u, Some(json!(["Reward", "Value"])), Some(json!("Value"))).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let receipt = svc.record(u, req(rev, json!("Reward"))).await.unwrap();

    assert_eq!(receipt.status, "incorrect");
    assert_eq!(receipt.assessment.unwrap().method, AssessmentMethod::Choice);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unkeyed_activity_stays_pending_with_assistance_recorded(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u, None, None).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let receipt = svc
        .record(u, RecordAttemptRequest {
            activity_revision_id: rev,
            response: json!("Because a smaller reward now can lead to larger rewards later."),
            assistance: vec![json!({"kind": "hint", "n": 1})],
        })
        .await
        .unwrap();

    assert_eq!(receipt.status, "pending");
    assert!(receipt.assessment.is_none());
    assert_eq!(count(&pool, "assessments").await, 0);

    let status: String = sqlx::query_scalar("SELECT status FROM attempt_status").fetch_one(&pool).await.unwrap();
    assert_eq!(status, "pending");
    let assistance: Value = sqlx::query_scalar("SELECT assistance FROM attempts").fetch_one(&pool).await.unwrap();
    assert_eq!(assistance, json!([{"kind": "hint", "n": 1}]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn foreign_revision_is_not_found_and_nothing_is_written(pool: PgPool) {
    let a = learner(&pool, "a@example.com").await;
    let b = learner(&pool, "b@example.com").await;
    let rev_a = revision(&pool, a, None, Some(json!("x"))).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let res = svc.record(b, req(rev_a, json!("x"))).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::NotFound(_)))), "{res:?}");

    let res = svc.record(a, req(Uuid::new_v4(), json!("x"))).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::NotFound(_)))), "{res:?}");

    assert_eq!(count(&pool, "attempts").await, 0);

    // And B cannot read A's receipt once one exists.
    let mine = svc.record(a, req(rev_a, json!("x"))).await.unwrap();
    let peek = svc.get(b, mine.attempt_id.parse().unwrap()).await;
    assert!(matches!(peek, Err(AppError::Domain(DomainError::NotFound(_)))));
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_response_is_rejected_before_any_write(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u, Some(json!(["Reward", "Value"])), Some(json!("Value"))).await;
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let res = svc.record(u, req(rev, json!("Policy"))).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::Validation(_)))), "{res:?}");

    let res = svc.record(u, req(rev, Value::Null)).await;
    assert!(matches!(res, Err(AppError::Validation(_))), "{res:?}");

    assert_eq!(count(&pool, "attempts").await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_failure_after_the_attempt_insert_rolls_everything_back(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let rev = revision(&pool, u, None, Some(json!("x"))).await;
    // Make the second write of the transaction fail.
    for ddl in [
        "CREATE FUNCTION explode() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'boom'; END; $$ LANGUAGE plpgsql",
        "CREATE TRIGGER explode_on_assess BEFORE INSERT ON assessments FOR EACH ROW EXECUTE FUNCTION explode()",
    ] {
        sqlx::query(ddl).execute(&pool).await.unwrap();
    }
    let svc = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let res = svc.record(u, req(rev, json!("x"))).await;
    assert!(matches!(res, Err(AppError::Domain(DomainError::Repository(_)))), "{res:?}");

    assert_eq!(count(&pool, "attempts").await, 0, "the attempt insert was rolled back with the failed assessment");
    assert_eq!(count(&pool, "assessments").await, 0);
}
