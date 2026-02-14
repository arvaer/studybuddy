use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::{QuizAnswer, QuizSession};
use domain::errors::DomainError;
use domain::repository_traits::QuizSessionRepository;

#[derive(Clone)]
pub struct PgQuizSessionRepository {
    pool: PgPool,
}

impl PgQuizSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl QuizSessionRepository for PgQuizSessionRepository {
    async fn create(
        &self,
        user_id: Uuid,
        config: serde_json::Value,
    ) -> Result<QuizSession, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO quiz_sessions (user_id, config)
            VALUES ($1, $2)
            RETURNING id, user_id, config, score, total_questions, started_at, completed_at
            "#,
            user_id,
            config,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(QuizSession {
            id:              row.id,
            user_id:         row.user_id,
            config:          row.config,
            score:           row.score,
            total_questions: row.total_questions,
            started_at:      row.started_at,
            completed_at:    row.completed_at,
        })
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<QuizSession, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, config, score, total_questions, started_at, completed_at
            FROM quiz_sessions WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("quiz session {id}")))?;

        Ok(QuizSession {
            id:              row.id,
            user_id:         row.user_id,
            config:          row.config,
            score:           row.score,
            total_questions: row.total_questions,
            started_at:      row.started_at,
            completed_at:    row.completed_at,
        })
    }

    async fn record_answer(
        &self,
        session_id: Uuid,
        question_id: Uuid,
        answer: &str,
        is_correct: bool,
    ) -> Result<QuizAnswer, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO quiz_answers (session_id, question_id, answer, is_correct)
            VALUES ($1, $2, $3, $4)
            RETURNING id, session_id, question_id, answer, is_correct, answered_at
            "#,
            session_id,
            question_id,
            answer,
            is_correct,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(QuizAnswer {
            id:          row.id,
            session_id:  row.session_id,
            question_id: row.question_id,
            answer:      row.answer,
            is_correct:  row.is_correct,
            answered_at: row.answered_at,
        })
    }

    async fn complete(
        &self,
        id: Uuid,
        user_id: Uuid,
        score: i32,
        total: i32,
    ) -> Result<QuizSession, DomainError> {
        let row = sqlx::query!(
            r#"
            UPDATE quiz_sessions SET
                score           = $3,
                total_questions = $4,
                completed_at    = now()
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, config, score, total_questions, started_at, completed_at
            "#,
            id,
            user_id,
            score,
            total,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("quiz session {id}")))?;

        Ok(QuizSession {
            id:              row.id,
            user_id:         row.user_id,
            config:          row.config,
            score:           row.score,
            total_questions: row.total_questions,
            started_at:      row.started_at,
            completed_at:    row.completed_at,
        })
    }

    async fn list_answers(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<QuizAnswer>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, session_id, question_id, answer, is_correct, answered_at
            FROM quiz_answers WHERE session_id = $1
            ORDER BY answered_at ASC
            "#,
            session_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| QuizAnswer {
                id:          r.id,
                session_id:  r.session_id,
                question_id: r.question_id,
                answer:      r.answer,
                is_correct:  r.is_correct,
                answered_at: r.answered_at,
            })
            .collect())
    }
}
