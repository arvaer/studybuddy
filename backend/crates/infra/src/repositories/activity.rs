//! Authoring and reading activities (#41). See docs/learning-records.md.

use std::ops::DerefMut;

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use domain::errors::DomainError;
use domain::learning::{
    Activity, ActivityKind, ActivityRevision, ActivityWithRevision, NewActivity, NewRevision, RevisionContent,
};
use domain::repository_traits::ActivityRepository;

use super::owned;

/// A connection for one repository call, however it was obtained.
pub type Conn<'a> = Box<dyn DerefMut<Target = PgConnection> + Send + 'a>;

/// Where a repository call gets its connection: the pool, one per call, or a
/// connection the caller already holds. The second is how the operator's
/// effect endpoint (19a) runs `create` on the transaction that also writes
/// the effect receipt, so the activity and the receipt land together or not
/// at all, while the manual route and the endpoint share every line of SQL.
pub trait Source: Send + Sync {
    fn conn(&self) -> impl std::future::Future<Output = Result<Conn<'_>, DomainError>> + Send;
}

impl Source for PgPool {
    async fn conn(&self) -> Result<Conn<'_>, DomainError> {
        Ok(Box::new(self.acquire().await.map_err(db)?))
    }
}

/// A connection the caller holds, lent to the repository for its lifetime.
/// `begin` on it opens a savepoint when the caller is mid-transaction, so
/// the repository's own commit is the caller's to keep or roll back.
pub struct Held<'c>(tokio::sync::Mutex<&'c mut PgConnection>);

struct HeldGuard<'a, 'c>(tokio::sync::MutexGuard<'a, &'c mut PgConnection>);

impl std::ops::Deref for HeldGuard<'_, '_> {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        &self.0
    }
}

impl DerefMut for HeldGuard<'_, '_> {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.0
    }
}

impl Source for Held<'_> {
    async fn conn(&self) -> Result<Conn<'_>, DomainError> {
        Ok(Box::new(HeldGuard(self.0.lock().await)))
    }
}

#[derive(Clone)]
pub struct PgActivityRepository<S: Source = PgPool> {
    source: S,
}

impl PgActivityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { source: pool }
    }
}

impl<'c> PgActivityRepository<Held<'c>> {
    /// A repository over a connection the caller holds, typically a
    /// transaction it will commit after writing more on it.
    pub fn held(conn: &'c mut PgConnection) -> Self {
        Self { source: Held(tokio::sync::Mutex::new(conn)) }
    }
}

fn db(e: sqlx::Error) -> DomainError {
    DomainError::Repository(e.to_string())
}

fn options_from(id: Uuid, v: Option<serde_json::Value>) -> Result<Option<Vec<String>>, DomainError> {
    v.map(serde_json::from_value)
        .transpose()
        .map_err(|e| DomainError::Repository(format!("activity revision {id} options are not string[]: {e}")))
}

/// Insert revision `number` of `activity_id` on the caller's connection.
async fn insert_revision(
    conn: &mut PgConnection,
    activity_id: Uuid,
    number: i32,
    c: &RevisionContent,
) -> Result<ActivityRevision, DomainError> {
    let options = c.options.as_ref().map(|o| serde_json::json!(o));
    let row = sqlx::query!(
        r#"
        INSERT INTO activity_revisions
            (activity_id, revision, prompt, options, answer_key, rubric,
             source_resource_id, source_artifact_id, source_location)
        VALUES ($1, $2, $3, $4, $5, $6, $7,
                (SELECT artifact_id FROM resources WHERE id = $7), $8)
        RETURNING id, source_artifact_id, created_at
        "#,
        activity_id,
        number,
        c.prompt.trim(),
        options,
        c.answer_key,
        c.rubric,
        c.source_resource_id,
        c.source_location,
    )
    .fetch_one(conn)
    .await
    .map_err(db)?;

    Ok(ActivityRevision {
        id:                 row.id,
        activity_id,
        revision:           number,
        prompt:             c.prompt.trim().to_string(),
        options:            c.options.clone(),
        answer_key:         c.answer_key.clone(),
        rubric:             c.rubric.clone(),
        source_resource_id: c.source_resource_id,
        source_artifact_id: row.source_artifact_id,
        source_location:    c.source_location.clone(),
        created_at:         row.created_at,
    })
}

/// One activity row joined to its highest-numbered revision.
struct CurrentRow {
    id:                 Uuid,
    user_id:            Uuid,
    concept_id:         Option<Uuid>,
    kind:               String,
    created_at:         chrono::DateTime<chrono::Utc>,
    r_id:               Uuid,
    r_revision:         i32,
    r_prompt:           String,
    r_options:          Option<serde_json::Value>,
    r_answer_key:       Option<serde_json::Value>,
    r_rubric:           Option<String>,
    r_source_resource_id: Option<Uuid>,
    r_source_artifact_id: Option<Uuid>,
    r_source_location:  Option<serde_json::Value>,
    r_created_at:       chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CurrentRow> for ActivityWithRevision {
    type Error = DomainError;

    fn try_from(r: CurrentRow) -> Result<Self, DomainError> {
        Ok(ActivityWithRevision {
            activity: Activity {
                id:         r.id,
                user_id:    r.user_id,
                concept_id: r.concept_id,
                kind:       ActivityKind::parse(&r.kind)
                    .ok_or_else(|| DomainError::Repository(format!("unknown activity kind {}", r.kind)))?,
                created_at: r.created_at,
            },
            current:  ActivityRevision {
                id:                 r.r_id,
                activity_id:        r.id,
                revision:           r.r_revision,
                prompt:             r.r_prompt,
                options:            options_from(r.r_id, r.r_options)?,
                answer_key:         r.r_answer_key,
                rubric:             r.r_rubric,
                source_resource_id: r.r_source_resource_id,
                source_artifact_id: r.r_source_artifact_id,
                source_location:    r.r_source_location,
                created_at:         r.r_created_at,
            },
        })
    }
}

impl<S: Source> PgActivityRepository<S> {
    /// `activity_id` narrows to one activity; `None` lists all of the owner's.
    async fn current(&self, user_id: Uuid, activity_id: Option<Uuid>) -> Result<Vec<ActivityWithRevision>, DomainError> {
        let mut conn = self.source.conn().await?;
        sqlx::query_as!(
            CurrentRow,
            r#"
            SELECT a.id, a.user_id, a.concept_id, a.kind::TEXT AS "kind!", a.created_at,
                   r.id AS r_id, r.revision AS r_revision, r.prompt AS r_prompt, r.options AS r_options,
                   r.answer_key AS r_answer_key, r.rubric AS r_rubric,
                   r.source_resource_id AS r_source_resource_id, r.source_artifact_id AS r_source_artifact_id,
                   r.source_location AS r_source_location,
                   r.created_at AS r_created_at
            FROM activities a
            JOIN LATERAL (
                SELECT * FROM activity_revisions WHERE activity_id = a.id ORDER BY revision DESC LIMIT 1
            ) r ON true
            WHERE a.user_id = $1 AND ($2::uuid IS NULL OR a.id = $2)
            ORDER BY a.created_at DESC, a.id
            "#,
            user_id,
            activity_id,
        )
        .fetch_all(&mut **conn)
        .await
        .map_err(db)?
        .into_iter()
        .map(ActivityWithRevision::try_from)
        .collect()
    }
}

impl<S: Source> ActivityRepository for PgActivityRepository<S> {
    async fn create(&self, cmd: NewActivity) -> Result<ActivityWithRevision, DomainError> {
        cmd.content.validate()?;
        let mut conn = self.source.conn().await?;
        // Linked ids must be the caller's. Ownership never changes, so the
        // check can precede the transaction (docs/ownership.md, point 3).
        owned::concepts(&mut **conn, cmd.user_id, cmd.concept_id.as_slice()).await?;
        owned::resource(&mut **conn, cmd.user_id, cmd.content.source_resource_id).await?;

        let mut tx = sqlx::Connection::begin(&mut **conn).await.map_err(db)?;
        let row = sqlx::query!(
            r#"
            INSERT INTO activities (user_id, concept_id, kind)
            VALUES ($1, $2, ($3::text)::activity_kind)
            RETURNING id, created_at
            "#,
            cmd.user_id,
            cmd.concept_id,
            cmd.kind.as_str(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let current = insert_revision(&mut tx, row.id, 1, &cmd.content).await?;
        tx.commit().await.map_err(db)?;

        Ok(ActivityWithRevision {
            activity: Activity {
                id:         row.id,
                user_id:    cmd.user_id,
                concept_id: cmd.concept_id,
                kind:       cmd.kind,
                created_at: row.created_at,
            },
            current,
        })
    }

    async fn revise(&self, cmd: NewRevision) -> Result<ActivityRevision, DomainError> {
        cmd.content.validate()?;
        let mut conn = self.source.conn().await?;
        owned::resource(&mut **conn, cmd.user_id, cmd.content.source_resource_id).await?;

        let mut tx = sqlx::Connection::begin(&mut **conn).await.map_err(db)?;
        // Ownership predicate and row lock first. Concurrent revisions of
        // the same activity queue on this lock.
        sqlx::query_scalar!(
            "SELECT id FROM activities WHERE id = $1 AND user_id = $2 FOR UPDATE",
            cmd.activity_id,
            cmd.user_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound(format!("activity {}", cmd.activity_id)))?;

        // The number must come from a separate statement: under READ
        // COMMITTED the snapshot is taken before the lock wait, so a max()
        // in the locking statement would read a stale revision count.
        let next = sqlx::query_scalar!(
            r#"SELECT max(revision) + 1 AS "next!" FROM activity_revisions WHERE activity_id = $1"#,
            cmd.activity_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;

        let revision = insert_revision(&mut tx, cmd.activity_id, next, &cmd.content).await?;
        tx.commit().await.map_err(db)?;
        Ok(revision)
    }

    async fn list(&self, user_id: Uuid) -> Result<Vec<ActivityWithRevision>, DomainError> {
        self.current(user_id, None).await
    }

    async fn find(&self, activity_id: Uuid, user_id: Uuid) -> Result<ActivityWithRevision, DomainError> {
        self.current(user_id, Some(activity_id))
            .await?
            .pop()
            .ok_or_else(|| DomainError::NotFound(format!("activity {activity_id}")))
    }

    async fn find_revision(&self, revision_id: Uuid, user_id: Uuid) -> Result<ActivityRevision, DomainError> {
        let mut conn = self.source.conn().await?;
        let row = sqlx::query!(
            r#"
            SELECT r.id, r.activity_id, r.revision, r.prompt, r.options, r.answer_key, r.rubric,
                   r.source_resource_id, r.source_artifact_id, r.source_location, r.created_at
            FROM activity_revisions r
            JOIN activities a ON a.id = r.activity_id
            WHERE r.id = $1 AND a.user_id = $2
            "#,
            revision_id,
            user_id,
        )
        .fetch_optional(&mut **conn)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound(format!("activity revision {revision_id}")))?;

        Ok(ActivityRevision {
            id:                 row.id,
            activity_id:        row.activity_id,
            revision:           row.revision,
            prompt:             row.prompt,
            options:            options_from(row.id, row.options)?,
            answer_key:         row.answer_key,
            rubric:             row.rubric,
            source_resource_id: row.source_resource_id,
            source_artifact_id: row.source_artifact_id,
            source_location:    row.source_location,
            created_at:         row.created_at,
        })
    }
}
