use uuid::Uuid;

use domain::entities::UserSettings;
use domain::repository_traits::SettingsRepository;

use crate::dtos::settings::{SettingsResponse, UpdateSettingsRequest};
use crate::errors::AppError;

pub struct SettingsService<R: SettingsRepository> {
    repo: R,
}

impl<R: SettingsRepository> SettingsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn get(&self, user_id: Uuid) -> Result<SettingsResponse, AppError> {
        let settings = self.repo.get(user_id).await?;
        Ok(SettingsResponse::from(settings))
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        req: UpdateSettingsRequest,
    ) -> Result<SettingsResponse, AppError> {
        // Fetch current (or use defaults if not yet created)
        let current = self.repo.get(user_id).await.unwrap_or_else(|_| {
            UserSettings {
                user_id,
                reinforcement_prompts: true,
                question_frequency:    50,
                ai_generated_notes:    true,
                study_time_goal:       60,
                daily_questions:       20,
                daily_reminders:       false,
                streak_alerts:         true,
                review_reminders:      true,
                reduce_animations:     false,
            }
        });

        let updated = req.apply_to(current, user_id);
        let saved   = self.repo.upsert(&updated).await?;
        Ok(SettingsResponse::from(saved))
    }
}
