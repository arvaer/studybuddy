use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::Question;
use domain::errors::DomainError;
use domain::repository_traits::QuestionRepository;
use domain::value_objects::QuestionType;

#[derive(Clone)]
pub struct PgQuestionRepository {
    pool: PgPool,
}

impl PgQuestionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl QuestionRepository for PgQuestionRepository {
    async fn create(
        &self,
        ru_id: Uuid,
        question_type: &str,
        prompt: &str,
        options: Option<Vec<String>>,
        correct_answer: &str,
        explanation: &str,
    ) -> Result<Question, DomainError> {
        let opts_json: Option<serde_json::Value> = options
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null));

        let row = sqlx::query(
            r#"
            INSERT INTO questions (ru_id, question_type, prompt, options, correct_answer, explanation)
            VALUES ($1, $2::question_type, $3, $4, $5, $6)
            RETURNING
                id, ru_id, question_type::TEXT AS question_type,
                prompt, options, correct_answer, explanation, created_at
            "#,
        )
        .bind(ru_id)
        .bind(question_type)
        .bind(prompt)
        .bind(opts_json)
        .bind(correct_answer)
        .bind(explanation)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| DomainError::Repository(e.to_string()))?;

        use sqlx::Row;
        let id: uuid::Uuid               = row.try_get("id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let ru_id_col: uuid::Uuid        = row.try_get("ru_id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let qt: Option<String>           = row.try_get("question_type").map_err(|e| DomainError::Repository(e.to_string()))?;
        let prompt_col: String           = row.try_get("prompt").map_err(|e| DomainError::Repository(e.to_string()))?;
        let opts: Option<serde_json::Value> = row.try_get("options").map_err(|e| DomainError::Repository(e.to_string()))?;
        let correct: String              = row.try_get("correct_answer").map_err(|e| DomainError::Repository(e.to_string()))?;
        let expl: String                 = row.try_get("explanation").map_err(|e| DomainError::Repository(e.to_string()))?;
        let created: chrono::DateTime<chrono::Utc> = row.try_get("created_at").map_err(|e| DomainError::Repository(e.to_string()))?;

        let options: Option<Vec<String>> = opts.and_then(|v| serde_json::from_value(v).ok());

        Ok(Question {
            id,
            ru_id: ru_id_col,
            question_type:  QuestionType::try_from(qt.as_deref().unwrap_or("recall"))?,
            prompt:         prompt_col,
            options,
            correct_answer: correct,
            explanation:    expl,
            created_at:     created,
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Question, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, ru_id, question_type::TEXT AS question_type,
                   prompt, options, correct_answer, explanation, created_at
            FROM questions WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("question {id}")))?;

        let options: Option<Vec<String>> = row.options.and_then(|v| serde_json::from_value(v).ok());

        Ok(Question {
            id:             row.id,
            ru_id:          row.ru_id,
            question_type:  QuestionType::try_from(row.question_type.as_deref().unwrap_or("recall"))?,
            prompt:         row.prompt,
            options,
            correct_answer: row.correct_answer,
            explanation:    row.explanation,
            created_at:     row.created_at,
        })
    }

    async fn list(
        &self,
        ru_id: Option<Uuid>,
        question_type: Option<&str>,
        concept_id: Option<Uuid>,
        topic_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<Vec<Question>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT q.id, q.ru_id,
                   q.question_type::TEXT AS question_type,
                   q.prompt, q.options, q.correct_answer, q.explanation, q.created_at
            FROM questions q
            JOIN reinforcement_units ru ON ru.id = q.ru_id
            JOIN concepts c ON c.id = ru.concept_id
            WHERE c.user_id = $1
              AND ($2::uuid IS NULL OR q.ru_id = $2)
              AND ($3::TEXT IS NULL OR q.question_type::TEXT = $3)
              AND ($4::uuid IS NULL OR c.id = $4)
              AND ($5::uuid IS NULL OR c.topic_id = $5)
            ORDER BY q.created_at DESC
            "#,
            user_id,
            ru_id,
            question_type,
            concept_id,
            topic_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                let options: Option<Vec<String>> =
                    r.options.and_then(|v| serde_json::from_value(v).ok());
                Ok(Question {
                    id:             r.id,
                    ru_id:          r.ru_id,
                    question_type:  QuestionType::try_from(r.question_type.as_deref().unwrap_or("recall"))?,
                    prompt:         r.prompt,
                    options,
                    correct_answer: r.correct_answer,
                    explanation:    r.explanation,
                    created_at:     r.created_at,
                })
            })
            .collect()
    }
}
