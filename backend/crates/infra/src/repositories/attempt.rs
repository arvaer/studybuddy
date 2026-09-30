//! Transactional recording of attempts (#9). See docs/learning-records.md.

use sqlx::PgPool;
use uuid::Uuid;

use domain::errors::DomainError;
use domain::learning::{
    self, ActivityRevision, Assessment, AssessmentMethod, AssessmentOutcome, AttemptReceipt, RecordAttempt,
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

impl AttemptRepository for PgAttemptRepository {
    async fn record(&self, cmd: RecordAttempt) -> Result<AttemptReceipt, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db)?;

        // Ownership predicate first: a foreign or missing revision is NotFound
        // and nothing below runs.
        let row = sqlx::query!(
            r#"
            SELECT r.id, r.activity_id, r.revision, r.prompt, r.options, r.answer_key, r.rubric
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
        };

        // The domain rule runs before any write, so a validation failure
        // costs nothing to roll back.
        let assessment = learning::assess(&revision, &cmd.response)?;

        let attempt = sqlx::query!(
            r#"
            INSERT INTO attempts (user_id, activity_revision_id, response, assistance)
            VALUES ($1, $2, $3, $4)
            RETURNING id, submitted_at
            "#,
            cmd.user_id,
            revision.id,
            cmd.response,
            serde_json::Value::Array(cmd.assistance),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;

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

        Ok(AttemptReceipt {
            attempt_id:           attempt.id,
            activity_revision_id: revision.id,
            submitted_at:         attempt.submitted_at,
            assessment,
        })
    }

    async fn find_receipt(&self, attempt_id: Uuid, user_id: Uuid) -> Result<AttemptReceipt, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT a.id, a.activity_revision_id, a.submitted_at,
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
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| DomainError::NotFound(format!("attempt {attempt_id}")))?;

        let assessment = match (row.outcome, row.method) {
            (Some(o), Some(m)) => Some(Assessment {
                outcome:  AssessmentOutcome::parse(&o)
                    .ok_or_else(|| DomainError::Repository(format!("unknown outcome {o}")))?,
                method:   AssessmentMethod::parse(&m)
                    .ok_or_else(|| DomainError::Repository(format!("unknown method {m}")))?,
                score:    row.score,
                feedback: row.feedback.unwrap_or_default(),
            }),
            _ => None,
        };

        Ok(AttemptReceipt {
            attempt_id:           row.id,
            activity_revision_id: row.activity_revision_id,
            submitted_at:         row.submitted_at,
            assessment,
        })
    }
}
