use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::Resource;
use domain::errors::DomainError;
use domain::repository_traits::ResourceRepository;
use domain::value_objects::ResourceType;

#[derive(Clone)]
pub struct PgResourceRepository {
    pool: PgPool,
}

impl PgResourceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn load_concept_ids(&self, resource_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query!(
            "SELECT concept_id FROM resource_concepts WHERE resource_id = $1",
            resource_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(rows.into_iter().map(|r| r.concept_id).collect())
    }
}

impl ResourceRepository for PgResourceRepository {
    async fn create(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        title: &str,
        resource_type: &str,
        url: Option<&str>,
        concept_ids: &[Uuid],
    ) -> Result<Resource, DomainError> {
        let row = sqlx::query(
            r#"
            INSERT INTO resources (user_id, topic_id, title, resource_type, url)
            VALUES ($1, $2, $3, $4::resource_type, $5)
            RETURNING id, user_id, topic_id, title, resource_type::TEXT AS resource_type,
                      url, added_at
            "#,
        )
        .bind(user_id)
        .bind(topic_id)
        .bind(title)
        .bind(resource_type)
        .bind(url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| DomainError::Repository(e.to_string()))?;

        use sqlx::Row;
        let row_id: Uuid               = row.try_get("id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_user: Uuid             = row.try_get("user_id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_topic: Uuid            = row.try_get("topic_id").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_title: String          = row.try_get("title").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_rt: Option<String>     = row.try_get("resource_type").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_url: Option<String>    = row.try_get("url").map_err(|e| DomainError::Repository(e.to_string()))?;
        let row_added: chrono::DateTime<chrono::Utc> = row.try_get("added_at").map_err(|e| DomainError::Repository(e.to_string()))?;

        for &cid in concept_ids {
            sqlx::query!(
                "INSERT INTO resource_concepts (resource_id, concept_id) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                row_id,
                cid,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        }

        Ok(Resource {
            id:            row_id,
            user_id:       row_user,
            topic_id:      row_topic,
            title:         row_title,
            resource_type: ResourceType::try_from(row_rt.as_deref().unwrap_or("article"))?,
            url:           row_url,
            concept_ids:   concept_ids.to_vec(),
            added_at:      row_added,
        })
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Resource, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, topic_id, title, resource_type::TEXT AS resource_type,
                   url, added_at
            FROM resources WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("resource {id}")))?;

        let concept_ids = self.load_concept_ids(row.id).await?;

        Ok(Resource {
            id:            row.id,
            user_id:       row.user_id,
            topic_id:      row.topic_id,
            title:         row.title,
            resource_type: ResourceType::try_from(row.resource_type.as_deref().unwrap_or("article"))?,
            url:           row.url,
            concept_ids,
            added_at:      row.added_at,
        })
    }

    async fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
        concept_id: Option<Uuid>,
        resource_type: Option<&str>,
    ) -> Result<Vec<Resource>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT r.id, r.user_id, r.topic_id, r.title,
                   r.resource_type::TEXT AS resource_type, r.url, r.added_at
            FROM resources r
            LEFT JOIN resource_concepts rc ON rc.resource_id = r.id
            WHERE r.user_id = $1
              AND ($2::uuid IS NULL OR r.topic_id = $2)
              AND ($3::uuid IS NULL OR rc.concept_id = $3)
              AND ($4::TEXT IS NULL OR r.resource_type::TEXT = $4)
            ORDER BY r.added_at DESC
            "#,
            user_id,
            topic_id,
            concept_id,
            resource_type,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        let mut resources = Vec::with_capacity(rows.len());
        for r in rows {
            let concept_ids = self.load_concept_ids(r.id).await?;
            resources.push(Resource {
                id:            r.id,
                user_id:       r.user_id,
                topic_id:      r.topic_id,
                title:         r.title,
                resource_type: ResourceType::try_from(r.resource_type.as_deref().unwrap_or("article"))?,
                url:           r.url,
                concept_ids,
                added_at:      r.added_at,
            });
        }
        Ok(resources)
    }

    async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            "DELETE FROM resources WHERE id = $1 AND user_id = $2",
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("resource {id}")));
        }
        Ok(())
    }
}
