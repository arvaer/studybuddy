//! The `capsule` schema and operator bookkeeping tables from 18a. The two
//! `capsule` tables must match what `capsule_host::postgres::PgStorage`
//! expects, since StudyBuddy opens it with `existing` and never lets it
//! create schema.

use sqlx::PgPool;

async fn columns(pool: &PgPool, schema: &str, table: &str) -> Vec<(String, String, String)> {
    sqlx::query_as(
        "SELECT column_name::text, data_type::text, is_nullable::text
         FROM information_schema.columns
         WHERE table_schema = $1 AND table_name = $2
         ORDER BY ordinal_position",
    )
    .bind(schema)
    .bind(table)
    .fetch_all(pool)
    .await
    .expect("information_schema")
}

fn s(a: &str, b: &str, c: &str) -> (String, String, String) {
    (a.into(), b.into(), c.into())
}

#[sqlx::test(migrations = "../../migrations")]
async fn capsule_tables_match_pg_storage(pool: PgPool) {
    assert_eq!(
        columns(&pool, "capsule", "nodes").await,
        vec![s("address", "text", "NO"), s("node", "jsonb", "NO")]
    );
    assert_eq!(
        columns(&pool, "capsule", "refs").await,
        vec![s("name", "text", "NO"), s("address", "text", "NO")]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn pg_storage_statements_run_against_the_schema(pool: PgPool) {
    // The exact statements PgStorage issues (host/src/postgres.rs at the pin).
    let node = serde_json::json!({"type": "session.form", "srcs": [], "body": {}, "tag": "cas-node-v0"});
    for _ in 0..2 {
        sqlx::query("INSERT INTO capsule.nodes (address, node) VALUES ($1, $2) ON CONFLICT (address) DO NOTHING")
            .bind("sha256:00")
            .bind(&node)
            .execute(&pool)
            .await
            .unwrap();
    }
    let created = sqlx::query("INSERT INTO capsule.refs (name, address) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING")
        .bind("refs/sessions/x")
        .bind("sha256:00")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(created.rows_affected(), 1);
    let swapped = sqlx::query("UPDATE capsule.refs SET address = $2 WHERE name = $1 AND address = $3")
        .bind("refs/sessions/x")
        .bind("sha256:01")
        .bind("sha256:00")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(swapped.rows_affected(), 1);
    let stale = sqlx::query("UPDATE capsule.refs SET address = $2 WHERE name = $1 AND address = $3")
        .bind("refs/sessions/x")
        .bind("sha256:02")
        .bind("sha256:00")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(stale.rows_affected(), 0, "a stale swap moves nothing");

    let rewrite = sqlx::query("UPDATE capsule.nodes SET node = $2 WHERE address = $1")
        .bind("sha256:00")
        .bind(&node)
        .execute(&pool)
        .await;
    assert!(rewrite.is_err(), "nodes are immutable");
}

#[sqlx::test(migrations = "../../migrations")]
async fn receipts_are_immutable_and_scoped_to_a_workspace(pool: PgPool) {
    let user: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind("a@example.com")
    .bind("x")
    .bind("A")
    .fetch_one(&pool)
    .await
    .unwrap();
    let ws: (uuid::Uuid,) = sqlx::query_as("INSERT INTO workspaces (user_id) VALUES ($1) RETURNING id")
        .bind(user.0)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO effect_receipts (effect_id, workspace_id, family, payload) VALUES ($1, $2, $3, $4)")
        .bind("effect-1")
        .bind(ws.0)
        .bind("learning.present")
        .bind(serde_json::json!({"revisionId": "r1"}))
        .execute(&pool)
        .await
        .unwrap();
    let dup = sqlx::query("INSERT INTO effect_receipts (effect_id, workspace_id, family, payload) VALUES ($1, $2, $3, $4)")
        .bind("effect-1")
        .bind(ws.0)
        .bind("learning.present")
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await;
    assert!(dup.is_err(), "same effect id is one receipt");
    let edit = sqlx::query("UPDATE effect_receipts SET payload = $2 WHERE effect_id = $1")
        .bind("effect-1")
        .bind(serde_json::json!({}))
        .execute(&pool)
        .await;
    assert!(edit.is_err(), "receipts are immutable");
    let half_lease = sqlx::query("INSERT INTO workspace_sessions (workspace_id, session_name, owner_lease) VALUES ($1, $2, gen_random_uuid())")
        .bind(ws.0)
        .bind("ws-1")
        .execute(&pool)
        .await;
    assert!(half_lease.is_err(), "a lease has both an owner and an expiry, or neither");
}
