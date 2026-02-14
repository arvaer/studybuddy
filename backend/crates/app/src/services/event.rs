use uuid::Uuid;

use domain::entities::Event;
use domain::repository_traits::EventRepository;

use crate::errors::AppError;

pub struct EventService<R: EventRepository> {
    repo: R,
}

impl<R: EventRepository> EventService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn append_event(
        &self,
        user_id: Uuid,
        event_type: &str,
        aggregate_type: &str,
        aggregate_id: Uuid,
        payload: serde_json::Value,
    ) -> Result<Event, AppError> {
        let event = Event {
            id:              Uuid::new_v4(),
            event_type:      event_type.to_string(),
            schema_version:  1,
            emitted_at:      chrono::Utc::now(),
            producer:        "backend".to_string(),
            user_id,
            aggregate_type:  aggregate_type.to_string(),
            aggregate_id,
            sequence:        0, // will be set by DB
            payload,
        };
        let saved = self.repo.append(&event).await?;
        Ok(saved)
    }
}
