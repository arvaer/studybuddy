//! Effect receipts (18c): what the application saw when it performed an
//! effect for the operator, kept by the effect id Core handed the provider
//! (sdk "Providers", H3: "the host keeps its receipts by that id").
//!
//! A receipt is written in the same transaction as the domain change it
//! reports (19a), so "the receipt exists" and "the domain committed" are one
//! fact. Rows are immutable (migration 20240107); a second write under the
//! same id is answered with the first row, never a second row.

use serde_json::Value as Json;
use sqlx::PgPool;
use uuid::Uuid;

/// One effect, performed once.
#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct Receipt {
    pub effect_id: String,
    pub workspace_id: Uuid,
    pub family: String,
    /// The observation the operator is completed with: exactly what the
    /// provider would have replied as `Reply::Value`.
    pub payload: Json,
}

/// What writing a receipt came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// This write landed.
    New,
    /// A receipt under this id was already there: the earlier observation,
    /// which is the one the operator must be completed with.
    Existing(Receipt),
}

/// Write `receipt` unless one exists under its id. Run it on the
/// transaction that performs the effect, so both land or neither does.
pub async fn record(
    conn: &mut sqlx::PgConnection,
    receipt: &Receipt,
) -> Result<Recorded, sqlx::Error> {
    let inserted = sqlx::query(
        "INSERT INTO effect_receipts (effect_id, workspace_id, family, payload)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (effect_id) DO NOTHING",
    )
    .bind(&receipt.effect_id)
    .bind(receipt.workspace_id)
    .bind(&receipt.family)
    .bind(&receipt.payload)
    .execute(&mut *conn)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(Recorded::New);
    }
    let existing = sqlx::query_as::<_, Receipt>(
        "SELECT effect_id, workspace_id, family, payload FROM effect_receipts WHERE effect_id = $1",
    )
    .bind(&receipt.effect_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(Recorded::Existing(existing))
}

/// The receipt for `effect_id` in `workspace`, if the effect was performed.
/// Scoped to the workspace: another workspace's receipt settles nothing here.
pub async fn find(
    pool: &PgPool,
    workspace: Uuid,
    effect_id: &str,
) -> Result<Option<Receipt>, sqlx::Error> {
    sqlx::query_as::<_, Receipt>(
        "SELECT effect_id, workspace_id, family, payload
         FROM effect_receipts WHERE effect_id = $1 AND workspace_id = $2",
    )
    .bind(effect_id)
    .bind(workspace)
    .fetch_optional(pool)
    .await
}
