use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::StudySession;
use domain::errors::DomainError;
use domain::repository_traits::StudySessionRepository;
use domain::value_objects::StudySessionType;

#[derive(Clone)]
pub struct PgStudySessionRepository {
    pool: PgPool,
}

impl PgStudySessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn load_concept_ids(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query!(
            "SELECT concept_id FROM study_session_concepts WHERE session_id = $1",
            session_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.concept_id).collect())
    }
}

impl StudySessionRepository for PgStudySessionRepository {
    async fn create(
        &self,
        user_id: Uuid,
        title: &str,
        session_type: &str,
        concept_ids: &[Uuid],
    ) -> Result<StudySession, DomainError> {
        let row = sqlx::query(
            r#"
            INSERT INTO study_sessions (user_id, title, session_type)
            VALUES ($1, $2, $3::study_session_type)
            RETURNING id, user_id, title, session_type::TEXT AS session_type,
                      progress, started_at, completed_at
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(session_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| DomainError::Repository(e.to_string()))?;

        use sqlx::Row;
        let row_id: Uuid              = row.try_get("id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_user: Uuid            = row.try_get("user_id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_title: String         = row.try_get("title").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_st: Option<String>    = row.try_get("session_type").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_prog: i32             = row.try_get("progress").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_started: chrono::DateTime<chrono::Utc> = row.try_get("started_at").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_completed: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at").map_err(|e| DomainError::Repository(e.to_string()))?;

        // Insert concept associations
        for &cid in concept_ids {
            sqlx::query!(
                "INSERT INTO study_session_concepts (session_id, concept_id) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                row_id,
                cid,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        }

        Ok(StudySession {
            id:           row_id,
            user_id:      row_user,
            title:        row_title,
            session_type: StudySessionType::try_from(row_st.as_deref().unwrap_or("reading"))?,
            concept_ids:  concept_ids.to_vec(),
            progress:     row_prog,
            started_at:   row_started,
            completed_at: row_completed,
        })
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<StudySession, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, title, session_type::TEXT AS session_type,
                   progress, started_at, completed_at
            FROM study_sessions WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("study session {id}")))?;

        let concept_ids = self.load_concept_ids(row.id).await?;

        Ok(StudySession {
            id:           row.id,
            user_id:      row.user_id,
            title:        row.title,
            session_type: StudySessionType::try_from(row.session_type.as_deref().unwrap_or("reading"))?,
            concept_ids,
            progress:     row.progress,
            started_at:   row.started_at,
            completed_at: row.completed_at,
        })
    }

    async fn list(
        &self,
        user_id: Uuid,
        completed: Option<bool>,
    ) -> Result<Vec<StudySession>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, title, session_type::TEXT AS session_type,
                   progress, started_at, completed_at
            FROM study_sessions
            WHERE user_id = $1
              AND ($2::bool IS NULL
                   OR ($2 = true  AND completed_at IS NOT NULL)
                   OR ($2 = false AND completed_at IS NULL))
            ORDER BY started_at DESC
            "#,
            user_id,
            completed,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        let mut sessions = Vec::with_capacity(rows.len());
        for r in rows {
            let concept_ids = self.load_concept_ids(r.id).await?;
            sessions.push(StudySession {
                id:           r.id,
                user_id:      r.user_id,
                title:        r.title,
                session_type: StudySessionType::try_from(r.session_type.as_deref().unwrap_or("reading"))?,
                concept_ids,
                progress:     r.progress,
                started_at:   r.started_at,
                completed_at: r.completed_at,
            });
        }
        Ok(sessions)
    }

    async fn update_progress(
        &self,
        id: Uuid,
        user_id: Uuid,
        progress: i32,
    ) -> Result<StudySession, DomainError> {
        let row = sqlx::query!(
            r#"
            UPDATE study_sessions SET progress = $3
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, title, session_type::TEXT AS session_type,
                      progress, started_at, completed_at
            "#,
            id,
            user_id,
            progress,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("study session {id}")))?;

        let concept_ids = self.load_concept_ids(row.id).await?;

        Ok(StudySession {
            id:           row.id,
            user_id:      row.user_id,
            title:        row.title,
            session_type: StudySessionType::try_from(row.session_type.as_deref().unwrap_or("reading"))?,
            concept_ids,
            progress:     row.progress,
            started_at:   row.started_at,
            completed_at: row.completed_at,
        })
    }

    async fn complete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<StudySession, DomainError> {
        let row = sqlx::query!(
            r#"
            UPDATE study_sessions SET progress = 100, completed_at = now()
            WHERE id = $1 AND user_id = $2 AND completed_at IS NULL
            RETURNING id, user_id, title, session_type::TEXT AS session_type,
                      progress, started_at, completed_at
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("study session {id}")))?;

        let concept_ids = self.load_concept_ids(row.id).await?;

        Ok(StudySession {
            id:           row.id,
            user_id:      row.user_id,
            title:        row.title,
            session_type: StudySessionType::try_from(row.session_type.as_deref().unwrap_or("reading"))?,
            concept_ids,
            progress:     row.progress,
            started_at:   row.started_at,
            completed_at: row.completed_at,
        })
    }
}
