//! Workspaces and goals (20a). A learner has one workspace in Phase 2,
//! made on first sight; a workspace holds one goal, at revision 1. Every
//! read and write is by the owning learner.

use chrono::{DateTime, Utc};
use domain::errors::DomainError;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub revision: i32,
    pub intent: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct PgWorkspaceRepository {
    pool: PgPool,
}

fn db(e: sqlx::Error) -> DomainError {
    DomainError::Repository(e.to_string())
}

impl PgWorkspaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The learner's workspace, made now if they have none.
    pub async fn find_or_create(&self, user_id: Uuid) -> Result<Uuid, DomainError> {
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM workspaces WHERE user_id = $1 ORDER BY created_at LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        if let Some((id,)) = existing {
            return Ok(id);
        }
        let (id,): (Uuid,) =
            sqlx::query_as("INSERT INTO workspaces (user_id) VALUES ($1) RETURNING id")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(db)?;
        Ok(id)
    }

    /// The learner's workspace, if they have one.
    pub async fn find(&self, user_id: Uuid) -> Result<Option<Uuid>, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM workspaces WHERE user_id = $1 ORDER BY created_at LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        Ok(row.map(|(id,)| id))
    }

    /// The workspace's owner.
    pub async fn owner(&self, workspace: Uuid) -> Result<Uuid, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as("SELECT user_id FROM workspaces WHERE id = $1")
            .bind(workspace)
            .fetch_optional(&self.pool)
            .await
            .map_err(db)?;
        row.map(|(id,)| id)
            .ok_or_else(|| DomainError::NotFound("workspace".into()))
    }

    /// The owner's first attempt against `revision`, if any: what a parked
    /// `learner/wait` is waiting for (20b).
    pub async fn first_attempt(
        &self,
        user_id: Uuid,
        revision: Uuid,
    ) -> Result<Option<Uuid>, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM attempts WHERE user_id = $1 AND activity_revision_id = $2
             ORDER BY submitted_at, id LIMIT 1",
        )
        .bind(user_id)
        .bind(revision)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        Ok(row.map(|(id,)| id))
    }

    /// `workspace` if `user_id` owns it, else `NotFound`: a learner is told
    /// nothing about another learner's workspace.
    pub async fn owned(&self, user_id: Uuid, workspace: Uuid) -> Result<Uuid, DomainError> {
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM workspaces WHERE id = $1 AND user_id = $2")
                .bind(workspace)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db)?;
        row.map(|(id,)| id)
            .ok_or_else(|| DomainError::NotFound("workspace".into()))
    }

    /// The workspace's goal at its latest revision, if any.
    pub async fn goal(&self, workspace: Uuid) -> Result<Option<Goal>, DomainError> {
        sqlx::query_as(
            "SELECT id, workspace_id, revision, intent, created_at FROM goals
             WHERE workspace_id = $1 ORDER BY revision DESC LIMIT 1",
        )
        .bind(workspace)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)
    }

    /// Store `intent` as the workspace's next goal revision. Whether a new
    /// goal may be set now (only when the operator is idle) is the route's
    /// check; the unique (workspace, revision) key settles a race.
    pub async fn set_goal(&self, workspace: Uuid, intent: &str) -> Result<Goal, DomainError> {
        let intent = intent.trim();
        if intent.is_empty() {
            return Err(DomainError::Validation("intent is required".into()));
        }
        if intent.chars().count() > 2000 {
            return Err(DomainError::Validation(
                "intent must be at most 2000 characters".into(),
            ));
        }
        sqlx::query_as(
            "INSERT INTO goals (workspace_id, revision, intent)
             SELECT $1, coalesce(max(revision), 0) + 1, $2 FROM goals WHERE workspace_id = $1
             RETURNING id, workspace_id, revision, intent, created_at",
        )
        .bind(workspace)
        .bind(intent)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => {
                DomainError::Conflict("a goal was just set for this workspace".into())
            }
            other => db(other),
        })
    }
}
