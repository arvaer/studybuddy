use chrono::Utc;
use uuid::Uuid;

use domain::repository_traits::ReinforcementUnitRepository;
use domain::value_objects::RuState;

use crate::dtos::reinforcement_unit::{CreateRuRequest, RuResponse, UpdateRuRequest};
use crate::errors::AppError;

pub struct RuService<R: ReinforcementUnitRepository> {
    repo: R,
}

impl<R: ReinforcementUnitRepository> RuService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(
        &self,
        req: CreateRuRequest,
    ) -> Result<Vec<RuResponse>, AppError> {
        let concept_id = Uuid::parse_str(&req.concept_id)
            .map_err(|_| AppError::Validation("invalid concept_id".into()))?;

        let mut created = Vec::with_capacity(req.items.len());
        for item in &req.items {
            let ru = self.repo.create(concept_id, &item.claim, &item.context).await?;
            created.push(RuResponse::from(ru));
        }
        Ok(created)
    }

    pub async fn list(
        &self,
        concept_id: Option<Uuid>,
        state: Option<&str>,
    ) -> Result<Vec<RuResponse>, AppError> {
        let rus = self.repo.list(concept_id, state).await?;
        Ok(rus.into_iter().map(RuResponse::from).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<RuResponse, AppError> {
        let ru = self.repo.find_by_id(id).await?;
        Ok(RuResponse::from(ru))
    }

    pub async fn update_after_review(
        &self,
        id: Uuid,
        req: UpdateRuRequest,
    ) -> Result<RuResponse, AppError> {
        let current = self.repo.find_by_id(id).await?;

        let new_state = if let Some(ref s) = req.state {
            RuState::try_from(s.as_str())
                .map_err(|e| AppError::Validation(e.to_string()))?
        } else {
            current.state
        };

        // SM-2 scheduling
        let stability = req.stability_score.unwrap_or(current.stability_score);
        let count     = req.reinforcement_count.unwrap_or(current.reinforcement_count);

        let (new_ease, new_interval) =
            compute_sm2(current.ease_factor, current.interval_days, stability);

        let due_at = Utc::now() + chrono::Duration::days(new_interval as i64);

        let updated = self
            .repo
            .update_after_review(
                id,
                &new_state.to_string(),
                stability,
                count,
                new_ease,
                new_interval,
                due_at,
            )
            .await?;

        Ok(RuResponse::from(updated))
    }
}

/// SM-2 variant: returns (new_ease_factor, new_interval_days).
/// `stability` is used as the answer quality (0.0–1.0 → 0–5 scale).
fn compute_sm2(ease: f64, interval: i32, stability: f64) -> (f64, i32) {
    // Map 0.0–1.0 stability to 0–5 quality score
    let q = (stability * 5.0).round() as i32;

    let new_ease = (ease + 0.1 - (5 - q) as f64 * (0.08 + (5 - q) as f64 * 0.02))
        .max(1.3);

    let new_interval = if q < 3 {
        1 // Reset on poor recall
    } else if interval == 0 {
        1
    } else if interval == 1 {
        6
    } else {
        (interval as f64 * new_ease).round() as i32
    };

    (new_ease, new_interval)
}
