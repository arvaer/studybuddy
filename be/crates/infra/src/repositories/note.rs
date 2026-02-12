use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::Note;
use domain::errors::DomainError;
use domain::repository_traits::NoteRepository;

#[derive(Clone)]
pub struct PgNoteRepository {
    pool: PgPool,
}

impl PgNoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl NoteRepository for PgNoteRepository {
    async fn create(
        &self,
        user_id: Uuid,
        concept_id: Uuid,
        ru_id: Option<Uuid>,
        content: &str,
        is_ai_generated: bool,
        anchor_position: Option<i32>,
    ) -> Result<Note, DomainError> {
        sqlx::query_as!(
            Note,
            r#"
            INSERT INTO notes (user_id, concept_id, ru_id, content, is_ai_generated, anchor_position)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, concept_id, ru_id, content, is_ai_generated,
                      anchor_position, created_at
            "#,
            user_id,
            concept_id,
            ru_id,
            content,
            is_ai_generated,
            anchor_position,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))
    }

    async fn find_by_id(&self, id: Uuid, user_id: Uuid) -> Result<Note, DomainError> {
        sqlx::query_as!(
            Note,
            r#"
            SELECT id, user_id, concept_id, ru_id, content, is_ai_generated,
                   anchor_position, created_at
            FROM notes WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("note {id}")))
    }

    async fn list(
        &self,
        user_id: Uuid,
        concept_id: Option<Uuid>,
        ru_id: Option<Uuid>,
    ) -> Result<Vec<Note>, DomainError> {
        sqlx::query_as!(
            Note,
            r#"
            SELECT id, user_id, concept_id, ru_id, content, is_ai_generated,
                   anchor_position, created_at
            FROM notes
            WHERE user_id = $1
              AND ($2::uuid IS NULL OR concept_id = $2)
              AND ($3::uuid IS NULL OR ru_id = $3)
            ORDER BY created_at DESC
            "#,
            user_id,
            concept_id,
            ru_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))
    }

    async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> Result<Note, DomainError> {
        sqlx::query_as!(
            Note,
            r#"
            UPDATE notes SET content = $3
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, concept_id, ru_id, content, is_ai_generated,
                      anchor_position, created_at
            "#,
            id,
            user_id,
            content,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("note {id}")))
    }

    async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            "DELETE FROM notes WHERE id = $1 AND user_id = $2",
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("note {id}")));
        }
        Ok(())
    }
}
