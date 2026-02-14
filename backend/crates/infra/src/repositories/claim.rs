use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::ClaimInContext;
use domain::errors::DomainError;
use domain::repository_traits::ClaimRepository;

#[derive(Clone)]
pub struct PgClaimRepository {
    pool: PgPool,
}

impl PgClaimRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ClaimRepository for PgClaimRepository {
    async fn create(
        &self,
        concept_id: Uuid,
        predicate: &str,
        supporting_text: &str,
        asset_id: Option<Uuid>,
        source_location: Option<serde_json::Value>,
    ) -> Result<ClaimInContext, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO claims_in_context (concept_id, predicate, supporting_text, asset_id, source_location)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, concept_id, predicate, supporting_text, asset_id, source_location, introduced_at
            "#,
            concept_id,
            predicate,
            supporting_text,
            asset_id,
            source_location,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(ClaimInContext {
            id:               row.id,
            concept_id:       row.concept_id,
            predicate:        row.predicate,
            supporting_text:  row.supporting_text,
            asset_id:         row.asset_id,
            source_location:  row.source_location,
            introduced_at:    row.introduced_at,
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<ClaimInContext, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, concept_id, predicate, supporting_text, asset_id, source_location, introduced_at
            FROM claims_in_context WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("claim {id}")))?;

        Ok(ClaimInContext {
            id:               row.id,
            concept_id:       row.concept_id,
            predicate:        row.predicate,
            supporting_text:  row.supporting_text,
            asset_id:         row.asset_id,
            source_location:  row.source_location,
            introduced_at:    row.introduced_at,
        })
    }

    async fn list_by_concept(&self, concept_id: Uuid) -> Result<Vec<ClaimInContext>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, concept_id, predicate, supporting_text, asset_id, source_location, introduced_at
            FROM claims_in_context WHERE concept_id = $1
            ORDER BY introduced_at ASC
            "#,
            concept_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ClaimInContext {
                id:               r.id,
                concept_id:       r.concept_id,
                predicate:        r.predicate,
                supporting_text:  r.supporting_text,
                asset_id:         r.asset_id,
                source_location:  r.source_location,
                introduced_at:    r.introduced_at,
            })
            .collect())
    }

    async fn list_by_asset(&self, asset_id: Uuid) -> Result<Vec<ClaimInContext>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, concept_id, predicate, supporting_text, asset_id, source_location, introduced_at
            FROM claims_in_context WHERE asset_id = $1
            ORDER BY introduced_at ASC
            "#,
            asset_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ClaimInContext {
                id:               r.id,
                concept_id:       r.concept_id,
                predicate:        r.predicate,
                supporting_text:  r.supporting_text,
                asset_id:         r.asset_id,
                source_location:  r.source_location,
                introduced_at:    r.introduced_at,
            })
            .collect())
    }
}
