use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::Topic;
use domain::errors::DomainError;
use domain::repository_traits::TopicRepository;

#[derive(Clone)]
pub struct PgTopicRepository {
    pool: PgPool,
}

impl PgTopicRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TopicRepository for PgTopicRepository {
    async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        description: &str,
        color: &str,
    ) -> Result<Topic, DomainError> {
        sqlx::query_as!(
            Topic,
            r#"
            INSERT INTO topics (user_id, name, description, color)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, name, description, color, created_at
            "#,
            user_id,
            name,
            description,
            color,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))
    }

    async fn find_by_id(&self, id: Uuid, user_id: Uuid) -> Result<Topic, DomainError> {
        sqlx::query_as!(
            Topic,
            r#"
            SELECT id, user_id, name, description, color, created_at
            FROM topics WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("topic {id}")))
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Topic>, DomainError> {
        sqlx::query_as!(
            Topic,
            r#"
            SELECT id, user_id, name, description, color, created_at
            FROM topics WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))
    }

    async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        color: Option<&str>,
    ) -> Result<Topic, DomainError> {
        sqlx::query_as!(
            Topic,
            r#"
            UPDATE topics SET
                name        = COALESCE($3, name),
                description = COALESCE($4, description),
                color       = COALESCE($5, color)
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, name, description, color, created_at
            "#,
            id,
            user_id,
            name,
            description,
            color,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("topic {id}")))
    }

    async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            "DELETE FROM topics WHERE id = $1 AND user_id = $2",
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("topic {id}")));
        }
        Ok(())
    }
}
