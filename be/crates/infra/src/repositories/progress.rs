use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::LearnerProgress;
use domain::errors::DomainError;
use domain::repository_traits::ProgressRepository;

#[derive(Clone)]
pub struct PgProgressRepository {
    pool: PgPool,
}

impl PgProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ProgressRepository for PgProgressRepository {
    async fn get_progress(&self, user_id: Uuid) -> Result<LearnerProgress, DomainError> {
        let row = sqlx::query!(
            r#"
            WITH
            concept_stats AS (
                SELECT
                    COUNT(*) AS total_concepts,
                    COUNT(*) FILTER (WHERE ru.state = 'stable') AS stable_concepts,
                    COUNT(*) FILTER (WHERE ru.state IN ('unstable', 'introduced')) AS needs_reinforcement
                FROM concepts c
                LEFT JOIN reinforcement_units ru ON ru.concept_id = c.id
                WHERE c.user_id = $1
            ),
            session_stats AS (
                SELECT
                    COUNT(*) FILTER (
                        WHERE completed_at >= now() - interval '7 days'
                    ) AS recent_sessions,
                    COALESCE(SUM(
                        EXTRACT(EPOCH FROM (
                            COALESCE(completed_at, started_at) - started_at
                        )) / 60.0
                    ), 0)::BIGINT AS total_study_time
                FROM study_sessions
                WHERE user_id = $1
            ),
            streak AS (
                -- Count consecutive days (most recent first) with at least one completed session
                SELECT COUNT(*) AS streak_days
                FROM (
                    SELECT DISTINCT DATE(completed_at) AS study_date
                    FROM study_sessions
                    WHERE user_id = $1
                      AND completed_at IS NOT NULL
                      AND completed_at >= now() - interval '365 days'
                ) dates
                WHERE study_date >= (
                    SELECT MAX(DATE(completed_at)) FROM study_sessions
                    WHERE user_id = $1 AND completed_at IS NOT NULL
                ) - (
                    SELECT COUNT(DISTINCT DATE(completed_at)) - 1
                    FROM study_sessions
                    WHERE user_id = $1
                      AND completed_at IS NOT NULL
                ) * interval '1 day'
            )
            SELECT
                COALESCE(cs.total_concepts, 0)      AS "total_concepts!: i64",
                COALESCE(cs.stable_concepts, 0)     AS "stable_concepts!: i64",
                COALESCE(cs.needs_reinforcement, 0) AS "needs_reinforcement!: i64",
                COALESCE(ss.recent_sessions, 0)     AS "recent_sessions!: i64",
                COALESCE(st.streak_days, 0)         AS "streak_days!: i64",
                COALESCE(ss.total_study_time, 0)    AS "total_study_time!: i64"
            FROM concept_stats cs
            CROSS JOIN session_stats ss
            CROSS JOIN streak st
            "#,
            user_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(LearnerProgress {
            total_concepts:      row.total_concepts,
            stable_concepts:     row.stable_concepts,
            needs_reinforcement: row.needs_reinforcement,
            recent_sessions:     row.recent_sessions,
            streak_days:         row.streak_days,
            total_study_time:    row.total_study_time,
        })
    }
}
