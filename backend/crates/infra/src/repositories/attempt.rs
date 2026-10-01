//! Transactional recording of attempts (#9) with idempotent submission by
//! request key (#10). See docs/learning-records.md.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use domain::errors::DomainError;
use domain::learning::{
    self, ActivityRevision, Assessment, AssessmentMethod, AssessmentOutcome, AttemptReceipt, RecordAttempt, Recorded,
};
use domain::repository_traits::AttemptRepository;

#[derive(Clone)]
pub struct PgAttemptRepository {
    pool: PgPool,
}

impl PgAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db(e: sqlx::Error) -> DomainError {
    DomainError::Repository(e.to_string())
}

impl PgAttemptRepository {
    /// The idempotency check (#10). If this learner already recorded
    /// `cmd.request_key`, replay that receipt when the payload is identical
    /// and refuse with `Conflict` otherwise. `None` means the key is unused.
    ///
    /// Everything runs on the one connection passed in. Acquiring a second
    /// one here while the caller holds a transaction would exhaust the pool
    /// under concurrent duplicates.
    async fn replay(conn: &mut PgConnection, cmd: &RecordAttempt) -> Result<Option<Recorded>, DomainError> {
        let existing = sqlx::query!(
            r#"
            SELECT id, activity_revision_id, response, assistance
            FROM attempts
            WHERE user_id = $1 AND request_key = $2
            "#,
            cmd.user_id,
            cmd.request_key,
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;

        let Some(existing) = existing else { return Ok(None) };
        if !cmd.same_payload(existing.activity_revision_id, &existing.response, &existing.assistance) {
            return Err(DomainError::Conflict(
                "request key was already used for a different submission".into(),
            ));
        }
        let receipt = receipt(conn, existing.id, cmd.user_id).await?;
        Ok(Some(Recorded { receipt, replayed: true }))
    }
}

/// The owner-scoped receipt read, on whichever connection the caller holds.
async fn receipt(conn: &mut PgConnection, attempt_id: Uuid, user_id: Uuid) -> Result<AttemptReceipt, DomainError> {
    let row = sqlx::query!(
        r#"
        SELECT a.id, a.activity_revision_id, a.submitted_at, a.response,
               s.outcome::TEXT AS "outcome?", s.method::TEXT AS "method?",
               s.score AS "score?", s.feedback AS "feedback?"
        FROM attempts a
        LEFT JOIN LATERAL (
            SELECT outcome, method, score, feedback FROM assessments
            WHERE attempt_id = a.id ORDER BY revision DESC LIMIT 1
        ) s ON true
        WHERE a.id = $1 AND a.user_id = $2
        "#,
        attempt_id,
        user_id,
    )
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(|| DomainError::NotFound(format!("attempt {attempt_id}")))?;

    let assessment = match (row.outcome, row.method) {
        (Some(o), Some(m)) => Some(Assessment {
            outcome:  AssessmentOutcome::parse(&o).ok_or_else(|| DomainError::Repository(format!("unknown outcome {o}")))?,
            method:   AssessmentMethod::parse(&m).ok_or_else(|| DomainError::Repository(format!("unknown method {m}")))?,
            score:    row.score,
            feedback: row.feedback.unwrap_or_default(),
        }),
        _ => None,
    };

    Ok(AttemptReceipt {
        attempt_id:           row.id,
        activity_revision_id: row.activity_revision_id,
        submitted_at:         row.submitted_at,
        response:             row.response,
        assessment,
    })
}

impl AttemptRepository for PgAttemptRepository {
    async fn record(&self, cmd: RecordAttempt) -> Result<Recorded, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;

        // A resent request key answers from the record before anything else
        // is checked or written.
        if let Some(replayed) = Self::replay(&mut tx, &cmd).await? {
            return Ok(replayed);
        }

        // Ownership predicate next: a foreign or missing revision is NotFound
        // and nothing below runs.
        let row = sqlx::query!(
            r#"
            SELECT r.id, r.activity_id, r.revision, r.prompt, r.options, r.answer_key, r.rubric,
                   r.source_resource_id, r.source_artifact_id, r.source_location, r.created_at
            FROM activity_revisions r
            JOIN activities a ON a.id = r.activity_id
            WHERE r.id = $1 AND a.user_id = $2
            "#,
            cmd.activity_revision_id,
            cmd.user_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound(format!("activity revision {}", cmd.activity_revision_id)))?;

        let revision = ActivityRevision {
            id:          row.id,
            activity_id: row.activity_id,
            revision:    row.revision,
            prompt:      row.prompt,
            options:     row.options.map(serde_json::from_value).transpose().map_err(|e| {
                DomainError::Repository(format!("activity revision {} options are not string[]: {e}", row.id))
            })?,
            answer_key:  row.answer_key,
            rubric:      row.rubric,
            source_resource_id: row.source_resource_id,
            source_artifact_id: row.source_artifact_id,
            source_location: row.source_location,
            created_at:  row.created_at,
        };

        // The domain rule runs before any write, so a validation failure
        // costs nothing to roll back.
        let assessment = learning::assess(&revision, &cmd.response)?;

        // `ON CONFLICT DO NOTHING` on the per-learner request-key index is
        // what makes concurrent duplicates safe: the second writer waits for
        // the first to commit, gets no row back, and replays instead.
        let attempt = sqlx::query!(
            r#"
            INSERT INTO attempts (user_id, request_key, activity_revision_id, response, assistance)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, request_key) WHERE request_key IS NOT NULL DO NOTHING
            RETURNING id, submitted_at
            "#,
            cmd.user_id,
            cmd.request_key,
            revision.id,
            cmd.response,
            serde_json::Value::Array(cmd.assistance.clone()),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;

        let Some(attempt) = attempt else {
            drop(tx); // rolls back; nothing of ours was written
            let mut conn = self.pool.acquire().await.map_err(db)?;
            return Self::replay(&mut conn, &cmd).await?.ok_or_else(|| {
                DomainError::Repository(format!("request key {} vanished during insert", cmd.request_key))
            });
        };

        if let Some(a) = &assessment {
            sqlx::query!(
                r#"
                INSERT INTO assessments (attempt_id, revision, outcome, method, score, feedback)
                VALUES ($1, 1, ($2::text)::assessment_outcome, ($3::text)::assessment_method, $4, $5)
                "#,
                attempt.id,
                a.outcome.as_str(),
                a.method.as_str(),
                a.score,
                a.feedback,
            )
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        tx.commit().await.map_err(db)?;

        Ok(Recorded {
            receipt:  AttemptReceipt {
                attempt_id:           attempt.id,
                activity_revision_id: revision.id,
                submitted_at:         attempt.submitted_at,
                response:             cmd.response,
                assessment,
            },
            replayed: false,
        })
    }

    async fn find_receipt(&self, attempt_id: Uuid, user_id: Uuid) -> Result<AttemptReceipt, DomainError> {
        let mut conn = self.pool.acquire().await.map_err(db)?;
        receipt(&mut conn, attempt_id, user_id).await
    }
}
