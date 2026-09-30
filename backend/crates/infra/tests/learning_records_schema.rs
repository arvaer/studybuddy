//! Schema rules from the learning-records migration (#8). Each test applies
//! every migration to an empty database first, which is the "applies from
//! empty" check.

use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn activity_with_revision(pool: &PgPool, user_id: Uuid) -> (Uuid, Uuid) {
    let activity: Uuid =
        sqlx::query_scalar("INSERT INTO activities (user_id, kind) VALUES ($1, 'recall') RETURNING id")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let revision: Uuid = sqlx::query_scalar(
        "INSERT INTO activity_revisions (activity_id, revision, prompt, answer_key)
         VALUES ($1, 1, 'What does a policy specify?', '\"a mapping from states to actions\"') RETURNING id",
    )
    .bind(activity)
    .fetch_one(pool)
    .await
    .unwrap();
    (activity, revision)
}

async fn attempt(pool: &PgPool, user_id: Uuid, revision: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO attempts (user_id, activity_revision_id, response) VALUES ($1, $2, '\"guess\"') RETURNING id",
    )
    .bind(user_id)
    .bind(revision)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn is_immutable_error(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(d) if d.message().contains("immutable"))
}

#[sqlx::test(migrations = "../../migrations")]
async fn revisions_are_numbered_uniquely_per_activity(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let (activity, _) = activity_with_revision(&pool, u).await;

    let dup = sqlx::query("INSERT INTO activity_revisions (activity_id, revision, prompt) VALUES ($1, 1, 'again')")
        .bind(activity)
        .execute(&pool)
        .await;
    assert!(matches!(dup, Err(sqlx::Error::Database(ref d)) if d.code().as_deref() == Some("23505")), "{dup:?}");

    sqlx::query("INSERT INTO activity_revisions (activity_id, revision, prompt) VALUES ($1, 2, 'edited')")
        .bind(activity)
        .execute(&pool)
        .await
        .expect("a new revision is how an edit is recorded");
}

#[sqlx::test(migrations = "../../migrations")]
async fn revisions_attempts_and_assessments_cannot_be_updated(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let (_, revision) = activity_with_revision(&pool, u).await;
    let attempt_id = attempt(&pool, u, revision).await;
    sqlx::query("INSERT INTO assessments (attempt_id, revision, outcome, method) VALUES ($1, 1, 'incorrect', 'exact_match')")
        .bind(attempt_id)
        .execute(&pool)
        .await
        .unwrap();

    let r = sqlx::query("UPDATE activity_revisions SET prompt = 'changed' WHERE id = $1").bind(revision).execute(&pool).await;
    assert!(is_immutable_error(&r.unwrap_err()));

    let r = sqlx::query("UPDATE attempts SET response = '\"better\"' WHERE id = $1").bind(attempt_id).execute(&pool).await;
    assert!(is_immutable_error(&r.unwrap_err()));

    let r = sqlx::query("UPDATE assessments SET outcome = 'correct' WHERE attempt_id = $1").bind(attempt_id).execute(&pool).await;
    assert!(is_immutable_error(&r.unwrap_err()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_unassessed_attempt_is_pending_not_incorrect(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let (_, revision) = activity_with_revision(&pool, u).await;
    let attempt_id = attempt(&pool, u, revision).await;

    let status: String = sqlx::query_scalar("SELECT status FROM attempt_status WHERE attempt_id = $1")
        .bind(attempt_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");

    sqlx::query("INSERT INTO assessments (attempt_id, revision, outcome, method) VALUES ($1, 1, 'incorrect', 'model')")
        .bind(attempt_id)
        .execute(&pool)
        .await
        .unwrap();
    // A correction is a new assessment revision, and it wins.
    sqlx::query("INSERT INTO assessments (attempt_id, revision, outcome, method) VALUES ($1, 2, 'correct', 'manual')")
        .bind(attempt_id)
        .execute(&pool)
        .await
        .unwrap();

    let status: String = sqlx::query_scalar("SELECT status FROM attempt_status WHERE attempt_id = $1")
        .bind(attempt_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "correct");
}

#[sqlx::test(migrations = "../../migrations")]
async fn request_key_is_unique_per_learner_only(pool: PgPool) {
    let a = learner(&pool, "a@example.com").await;
    let b = learner(&pool, "b@example.com").await;
    let (_, rev_a) = activity_with_revision(&pool, a).await;
    let (_, rev_b) = activity_with_revision(&pool, b).await;

    let insert = |user: Uuid, rev: Uuid| {
        sqlx::query("INSERT INTO attempts (user_id, activity_revision_id, response, request_key) VALUES ($1, $2, '\"x\"', 'key-1')")
            .bind(user)
            .bind(rev)
            .execute(&pool)
    };
    insert(a, rev_a).await.unwrap();
    insert(b, rev_b).await.expect("another learner may reuse the key");
    let dup = insert(a, rev_a).await;
    assert!(matches!(dup, Err(sqlx::Error::Database(ref d)) if d.code().as_deref() == Some("23505")), "{dup:?}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_a_learner_removes_their_records(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let (_, revision) = activity_with_revision(&pool, u).await;
    let attempt_id = attempt(&pool, u, revision).await;
    sqlx::query("INSERT INTO assessments (attempt_id, revision, outcome, method) VALUES ($1, 1, 'correct', 'choice')")
        .bind(attempt_id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM users WHERE id = $1").bind(u).execute(&pool).await.expect("cascade is not blocked by the immutability trigger");

    for table in ["activities", "activity_revisions", "attempts", "assessments"] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0, "{table}");
    }
}
