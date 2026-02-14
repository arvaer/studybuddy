use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::Event;
use domain::errors::DomainError;
use domain::repository_traits::EventRepository;

#[derive(Clone)]
pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl EventRepository for PgEventRepository {
    async fn append(&self, event: &Event) -> Result<Event, DomainError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO events (event_type, schema_version, producer, user_id, aggregate_type, aggregate_id, payload)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, event_type, schema_version, emitted_at, producer, user_id,
                      aggregate_type, aggregate_id, sequence, payload
            "#,
            event.event_type,
            event.schema_version,
            event.producer,
            event.user_id,
            event.aggregate_type,
            event.aggregate_id,
            event.payload,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(Event {
            id:              row.id,
            event_type:      row.event_type,
            schema_version:  row.schema_version,
            emitted_at:      row.emitted_at,
            producer:        row.producer,
            user_id:         row.user_id,
            aggregate_type:  row.aggregate_type,
            aggregate_id:    row.aggregate_id,
            sequence:        row.sequence,
            payload:         row.payload,
        })
    }

    async fn list_by_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> Result<Vec<Event>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, event_type, schema_version, emitted_at, producer, user_id,
                   aggregate_type, aggregate_id, sequence, payload
            FROM events
            WHERE aggregate_type = $1 AND aggregate_id = $2
            ORDER BY sequence ASC
            "#,
            aggregate_type,
            aggregate_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| Event {
                id:              r.id,
                event_type:      r.event_type,
                schema_version:  r.schema_version,
                emitted_at:      r.emitted_at,
                producer:        r.producer,
                user_id:         r.user_id,
                aggregate_type:  r.aggregate_type,
                aggregate_id:    r.aggregate_id,
                sequence:        r.sequence,
                payload:         r.payload,
            })
            .collect())
    }
}
