use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::{Concept, ReinforcementUnit};
use domain::errors::DomainError;
use domain::repository_traits::ConceptRepository;
use domain::value_objects::RuState;

#[derive(Clone)]
pub struct PgConceptRepository {
    pool: PgPool,
}

impl PgConceptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn load_rus(&self, concept_id: Uuid) -> Result<Vec<ReinforcementUnit>, DomainError> {
        // Load RUs with their dependency arrays
        let rows = sqlx::query!(
            r#"
            SELECT
                ru.id,
                ru.concept_id,
                ru.claim,
                ru.context,
                ru.claim_id,
                ru.dependency_cost,
                ru.state::TEXT AS state,
                ru.stability_score,
                ru.reinforcement_count,
                ru.last_reinforced,
                ru.ease_factor,
                ru.interval_days,
                ru.due_at,
                ru.source_resource_id,
                ru.created_at,
                COALESCE(
                    array_agg(rd.depends_on_id) FILTER (WHERE rd.depends_on_id IS NOT NULL),
                    '{}'
                ) AS "dependencies!"
            FROM reinforcement_units ru
            LEFT JOIN ru_dependencies rd ON rd.ru_id = ru.id
            WHERE ru.concept_id = $1
            GROUP BY ru.id
            ORDER BY ru.created_at ASC
            "#,
            concept_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                Ok(ReinforcementUnit {
                    id:                  r.id,
                    concept_id:          r.concept_id,
                    claim:               r.claim,
                    context:             r.context,
                    claim_id:            r.claim_id,
                    dependency_cost:     r.dependency_cost,
                    state:               RuState::try_from(r.state.as_deref().unwrap_or("introduced"))?,
                    stability_score:     r.stability_score,
                    reinforcement_count: r.reinforcement_count,
                    last_reinforced:     r.last_reinforced,
                    ease_factor:         r.ease_factor,
                    interval_days:       r.interval_days,
                    due_at:              r.due_at,
                    dependencies:        r.dependencies,
                    source_resource_id:  r.source_resource_id,
                    created_at:          r.created_at,
                })
            })
            .collect()
    }
}

impl ConceptRepository for PgConceptRepository {
    async fn create(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
        parent_id: Option<Uuid>,
        name: &str,
        description: &str,
    ) -> Result<Concept, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO concepts (user_id, topic_id, parent_id, name, description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, topic_id, parent_id, name, description, created_at
            "#,
            user_id,
            topic_id,
            parent_id,
            name,
            description,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(Concept {
            id:                  row.id,
            user_id:             row.user_id,
            topic_id:            row.topic_id,
            parent_id:           row.parent_id,
            name:                row.name,
            description:         row.description,
            reinforcement_units: vec![],
            created_at:          row.created_at,
        })
    }

    async fn find_by_id(&self, id: Uuid, user_id: Uuid) -> Result<Concept, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, topic_id, parent_id, name, description, created_at
            FROM concepts WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("concept {id}")))?;

        let rus = self.load_rus(row.id).await?;

        Ok(Concept {
            id:                  row.id,
            user_id:             row.user_id,
            topic_id:            row.topic_id,
            parent_id:           row.parent_id,
            name:                row.name,
            description:         row.description,
            reinforcement_units: rus,
            created_at:          row.created_at,
        })
    }

    async fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
    ) -> Result<Vec<Concept>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, topic_id, parent_id, name, description, created_at
            FROM concepts
            WHERE user_id = $1
              AND ($2::uuid IS NULL OR topic_id = $2)
            ORDER BY created_at DESC
            "#,
            user_id,
            topic_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        let mut concepts = Vec::with_capacity(rows.len());
        for row in rows {
            let rus = self.load_rus(row.id).await?;
            concepts.push(Concept {
                id:                  row.id,
                user_id:             row.user_id,
                topic_id:            row.topic_id,
                parent_id:           row.parent_id,
                name:                row.name,
                description:         row.description,
                reinforcement_units: rus,
                created_at:          row.created_at,
            });
        }
        Ok(concepts)
    }

    async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        topic_id: Option<Option<Uuid>>,
        parent_id: Option<Option<Uuid>>,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Concept, DomainError> {
        let row = sqlx::query!(
            r#"
            UPDATE concepts SET
                name        = COALESCE($3, name),
                description = COALESCE($4, description),
                topic_id    = CASE WHEN $5 THEN $6 ELSE topic_id END,
                parent_id   = CASE WHEN $7 THEN $8 ELSE parent_id END
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, topic_id, parent_id, name, description, created_at
            "#,
            id,
            user_id,
            name,
            description,
            topic_id.is_some(),
            topic_id.flatten(),
            parent_id.is_some(),
            parent_id.flatten(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("concept {id}")))?;

        let rus = self.load_rus(row.id).await?;

        Ok(Concept {
            id:                  row.id,
            user_id:             row.user_id,
            topic_id:            row.topic_id,
            parent_id:           row.parent_id,
            name:                row.name,
            description:         row.description,
            reinforcement_units: rus,
            created_at:          row.created_at,
        })
    }

    async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query!(
            "DELETE FROM concepts WHERE id = $1 AND user_id = $2",
            id,
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("concept {id}")));
        }
        Ok(())
    }
}
