use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::ReinforcementUnit;
use domain::errors::DomainError;
use domain::repository_traits::ReinforcementUnitRepository;
use domain::value_objects::RuState;

#[derive(Clone)]
pub struct PgRuRepository {
    pool: PgPool,
}

impl PgRuRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_row(
        id: Uuid,
        concept_id: Uuid,
        claim: String,
        context: String,
        claim_id: Option<Uuid>,
        dependency_cost: f64,
        state: Option<String>,
        stability_score: f64,
        reinforcement_count: i32,
        last_reinforced: Option<DateTime<Utc>>,
        ease_factor: f64,
        interval_days: i32,
        due_at: DateTime<Utc>,
        dependencies: Vec<Uuid>,
        source_resource_id: Option<Uuid>,
        created_at: DateTime<Utc>,
    ) -> Result<ReinforcementUnit, DomainError> {
        Ok(ReinforcementUnit {
            id,
            concept_id,
            claim,
            context,
            claim_id,
            dependency_cost,
            state: RuState::try_from(state.as_deref().unwrap_or("introduced"))?,
            stability_score,
            reinforcement_count,
            last_reinforced,
            ease_factor,
            interval_days,
            due_at,
            dependencies,
            source_resource_id,
            created_at,
        })
    }
}

impl ReinforcementUnitRepository for PgRuRepository {
    async fn create(
        &self,
        concept_id: Uuid,
        claim: &str,
        context: &str,
    ) -> Result<ReinforcementUnit, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO reinforcement_units (concept_id, claim, context)
            VALUES ($1, $2, $3)
            RETURNING
                id, concept_id, claim, context, claim_id, dependency_cost,
                state::TEXT AS state,
                stability_score, reinforcement_count, last_reinforced,
                ease_factor, interval_days, due_at, source_resource_id, created_at
            "#,
            concept_id,
            claim,
            context,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Self::map_row(
            row.id, row.concept_id, row.claim, row.context,
            row.claim_id, row.dependency_cost,
            row.state,
            row.stability_score, row.reinforcement_count, row.last_reinforced,
            row.ease_factor, row.interval_days, row.due_at, vec![],
            row.source_resource_id, row.created_at,
        )
    }

    async fn create_with_source(
        &self,
        concept_id: Uuid,
        claim: &str,
        context: &str,
        source_resource_id: Option<Uuid>,
        claim_id: Option<Uuid>,
    ) -> Result<ReinforcementUnit, DomainError> {
        let row = sqlx::query(
            r#"
            INSERT INTO reinforcement_units (concept_id, claim, context, source_resource_id, claim_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, concept_id, claim, context, claim_id, dependency_cost,
                state::TEXT AS state,
                stability_score, reinforcement_count, last_reinforced,
                ease_factor, interval_days, due_at, source_resource_id, created_at
            "#,
        )
        .bind(concept_id)
        .bind(claim)
        .bind(context)
        .bind(source_resource_id)
        .bind(claim_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: sqlx::Error| DomainError::Repository(e.to_string()))?;

        use sqlx::Row;
        Self::map_row(
            row.try_get("id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("concept_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("claim").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("context").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("claim_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("dependency_cost").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("state").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("stability_score").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("reinforcement_count").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("last_reinforced").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("ease_factor").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("interval_days").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("due_at").map_err(|e| DomainError::Repository(e.to_string()))?,
            vec![],
            row.try_get("source_resource_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("created_at").map_err(|e| DomainError::Repository(e.to_string()))?,
        )
    }

    async fn find_by_id(&self, id: Uuid) -> Result<ReinforcementUnit, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT
                ru.id, ru.concept_id, ru.claim, ru.context,
                ru.claim_id, ru.dependency_cost,
                ru.state::TEXT AS state,
                ru.stability_score, ru.reinforcement_count, ru.last_reinforced,
                ru.ease_factor, ru.interval_days, ru.due_at, ru.source_resource_id, ru.created_at,
                COALESCE(
                    array_agg(rd.depends_on_id) FILTER (WHERE rd.depends_on_id IS NOT NULL),
                    '{}'
                ) AS "dependencies!"
            FROM reinforcement_units ru
            LEFT JOIN ru_dependencies rd ON rd.ru_id = ru.id
            WHERE ru.id = $1
            GROUP BY ru.id
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("ru {id}")))?;

        Self::map_row(
            row.id, row.concept_id, row.claim, row.context,
            row.claim_id, row.dependency_cost,
            row.state,
            row.stability_score, row.reinforcement_count, row.last_reinforced,
            row.ease_factor, row.interval_days, row.due_at, row.dependencies,
            row.source_resource_id, row.created_at,
        )
    }

    async fn list(
        &self,
        concept_id: Option<Uuid>,
        state: Option<&str>,
    ) -> Result<Vec<ReinforcementUnit>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                ru.id, ru.concept_id, ru.claim, ru.context,
                ru.claim_id, ru.dependency_cost,
                ru.state::TEXT AS state,
                ru.stability_score, ru.reinforcement_count, ru.last_reinforced,
                ru.ease_factor, ru.interval_days, ru.due_at, ru.source_resource_id, ru.created_at,
                COALESCE(
                    array_agg(rd.depends_on_id) FILTER (WHERE rd.depends_on_id IS NOT NULL),
                    '{}'
                ) AS "dependencies!"
            FROM reinforcement_units ru
            LEFT JOIN ru_dependencies rd ON rd.ru_id = ru.id
            WHERE ($1::uuid IS NULL OR ru.concept_id = $1)
              AND ($2::TEXT IS NULL OR ru.state::TEXT = $2)
            GROUP BY ru.id
            ORDER BY ru.created_at ASC
            "#,
            concept_id,
            state,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                Self::map_row(
                    r.id, r.concept_id, r.claim, r.context,
                    r.claim_id, r.dependency_cost,
                    r.state,
                    r.stability_score, r.reinforcement_count, r.last_reinforced,
                    r.ease_factor, r.interval_days, r.due_at, r.dependencies,
                    r.source_resource_id, r.created_at,
                )
            })
            .collect()
    }

    async fn update_after_review(
        &self,
        id: Uuid,
        state: &str,
        stability_score: f64,
        reinforcement_count: i32,
        ease_factor: f64,
        interval_days: i32,
        due_at: DateTime<Utc>,
    ) -> Result<ReinforcementUnit, DomainError> {
        let row = sqlx::query(
            r#"
            UPDATE reinforcement_units SET
                state               = $2::ru_state,
                stability_score     = $3,
                reinforcement_count = $4,
                ease_factor         = $5,
                interval_days       = $6,
                due_at              = $7,
                last_reinforced     = now()
            WHERE id = $1
            RETURNING
                id, concept_id, claim, context, claim_id, dependency_cost,
                state::TEXT AS state,
                stability_score, reinforcement_count, last_reinforced,
                ease_factor, interval_days, due_at, source_resource_id, created_at
            "#,
        )
        .bind(id)
        .bind(state)
        .bind(stability_score)
        .bind(reinforcement_count)
        .bind(ease_factor)
        .bind(interval_days)
        .bind(due_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: sqlx::Error| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("ru {id}")))?;

        use sqlx::Row;
        Self::map_row(
            row.try_get("id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("concept_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("claim").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("context").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("claim_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("dependency_cost").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("state").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("stability_score").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("reinforcement_count").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("last_reinforced").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("ease_factor").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("interval_days").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("due_at").map_err(|e| DomainError::Repository(e.to_string()))?,
            vec![],
            row.try_get("source_resource_id").map_err(|e| DomainError::Repository(e.to_string()))?,
            row.try_get("created_at").map_err(|e| DomainError::Repository(e.to_string()))?,
        )
    }

    async fn list_due(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ReinforcementUnit>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                ru.id, ru.concept_id, ru.claim, ru.context,
                ru.claim_id, ru.dependency_cost,
                ru.state::TEXT AS state,
                ru.stability_score, ru.reinforcement_count, ru.last_reinforced,
                ru.ease_factor, ru.interval_days, ru.due_at, ru.source_resource_id, ru.created_at,
                COALESCE(
                    array_agg(rd.depends_on_id) FILTER (WHERE rd.depends_on_id IS NOT NULL),
                    '{}'
                ) AS "dependencies!"
            FROM reinforcement_units ru
            JOIN concepts c ON c.id = ru.concept_id
            LEFT JOIN ru_dependencies rd ON rd.ru_id = ru.id
            WHERE c.user_id = $1
              AND ru.due_at <= now()
              AND ru.state::TEXT NOT IN ('stable', 'superseded')
            GROUP BY ru.id
            ORDER BY ru.due_at ASC
            LIMIT $2
            "#,
            user_id,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        rows.into_iter()
            .map(|r| {
                Self::map_row(
                    r.id, r.concept_id, r.claim, r.context,
                    r.claim_id, r.dependency_cost,
                    r.state,
                    r.stability_score, r.reinforcement_count, r.last_reinforced,
                    r.ease_factor, r.interval_days, r.due_at, r.dependencies,
                    r.source_resource_id, r.created_at,
                )
            })
            .collect()
    }
}
