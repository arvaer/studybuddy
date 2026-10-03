//! Linked-ID ownership checks (docs/ownership.md, convention point 3).
//!
//! A write that stores another record's id must prove the caller owns that
//! record. Single links use `INSERT ... SELECT` (see `reinforcement_unit.rs`);
//! multi-id links and optional links call these helpers first. A missing or
//! foreign id is `NotFound`, indistinguishable from a nonexistent one.
//! Ownership never changes, so a check-then-insert has no race to exploit.

use sqlx::PgExecutor;
use uuid::Uuid;

use domain::errors::DomainError;

fn distinct(ids: &[Uuid]) -> Vec<Uuid> {
    let mut v = ids.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

fn check(kind: &str, ids: &[Uuid], found: i64) -> Result<(), DomainError> {
    if found == ids.len() as i64 {
        Ok(())
    } else {
        Err(DomainError::NotFound(format!("{kind} {ids:?}")))
    }
}

/// Every id in `ids` is a concept owned by `user_id`.
pub async fn concepts(exec: impl PgExecutor<'_>, user_id: Uuid, ids: &[Uuid]) -> Result<(), DomainError> {
    let ids = distinct(ids);
    if ids.is_empty() {
        return Ok(());
    }
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM concepts WHERE user_id = $1 AND id = ANY($2)"#,
        user_id,
        &ids,
    )
    .fetch_one(exec)
    .await
    .map_err(|e| DomainError::Repository(e.to_string()))?;
    check("concept", &ids, n)
}

/// `topic_id`, if given, is a topic owned by `user_id`.
pub async fn topic(exec: impl PgExecutor<'_>, user_id: Uuid, topic_id: Option<Uuid>) -> Result<(), DomainError> {
    let Some(id) = topic_id else { return Ok(()) };
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM topics WHERE user_id = $1 AND id = $2"#,
        user_id,
        id,
    )
    .fetch_one(exec)
    .await
    .map_err(|e| DomainError::Repository(e.to_string()))?;
    check("topic", &[id], n)
}

/// `resource_id`, if given, is a resource owned by `user_id`.
pub async fn resource(exec: impl PgExecutor<'_>, user_id: Uuid, resource_id: Option<Uuid>) -> Result<(), DomainError> {
    let Some(id) = resource_id else { return Ok(()) };
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM resources WHERE user_id = $1 AND id = $2"#,
        user_id,
        id,
    )
    .fetch_one(exec)
    .await
    .map_err(|e| DomainError::Repository(e.to_string()))?;
    check("resource", &[id], n)
}

/// `ru_id`, if given, is a reinforcement unit whose concept `user_id` owns.
pub async fn reinforcement_unit(exec: impl PgExecutor<'_>, user_id: Uuid, ru_id: Option<Uuid>) -> Result<(), DomainError> {
    let Some(id) = ru_id else { return Ok(()) };
    let n = sqlx::query_scalar!(
        r#"
        SELECT count(*) AS "n!"
        FROM reinforcement_units ru
        JOIN concepts c ON c.id = ru.concept_id
        WHERE c.user_id = $1 AND ru.id = $2
        "#,
        user_id,
        id,
    )
    .fetch_one(exec)
    .await
    .map_err(|e| DomainError::Repository(e.to_string()))?;
    check("reinforcement unit", &[id], n)
}
