use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::UserSettings;
use domain::errors::DomainError;
use domain::repository_traits::SettingsRepository;

#[derive(Clone)]
pub struct PgSettingsRepository {
    pool: PgPool,
}

impl PgSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SettingsRepository for PgSettingsRepository {
    async fn get(&self, user_id: Uuid) -> Result<UserSettings, DomainError> {
        sqlx::query_as!(
            UserSettings,
            r#"
            SELECT user_id, reinforcement_prompts, question_frequency, ai_generated_notes,
                   study_time_goal, daily_questions, daily_reminders, streak_alerts,
                   review_reminders, reduce_animations
            FROM user_settings WHERE user_id = $1
            "#,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("settings for user {user_id}")))
    }

    async fn upsert(&self, s: &UserSettings) -> Result<UserSettings, DomainError> {
        sqlx::query_as!(
            UserSettings,
            r#"
            INSERT INTO user_settings (
                user_id, reinforcement_prompts, question_frequency, ai_generated_notes,
                study_time_goal, daily_questions, daily_reminders, streak_alerts,
                review_reminders, reduce_animations
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (user_id) DO UPDATE SET
                reinforcement_prompts = EXCLUDED.reinforcement_prompts,
                question_frequency    = EXCLUDED.question_frequency,
                ai_generated_notes    = EXCLUDED.ai_generated_notes,
                study_time_goal       = EXCLUDED.study_time_goal,
                daily_questions       = EXCLUDED.daily_questions,
                daily_reminders       = EXCLUDED.daily_reminders,
                streak_alerts         = EXCLUDED.streak_alerts,
                review_reminders      = EXCLUDED.review_reminders,
                reduce_animations     = EXCLUDED.reduce_animations
            RETURNING
                user_id, reinforcement_prompts, question_frequency, ai_generated_notes,
                study_time_goal, daily_questions, daily_reminders, streak_alerts,
                review_reminders, reduce_animations
            "#,
            s.user_id,
            s.reinforcement_prompts,
            s.question_frequency,
            s.ai_generated_notes,
            s.study_time_goal,
            s.daily_questions,
            s.daily_reminders,
            s.streak_alerts,
            s.review_reminders,
            s.reduce_animations,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))
    }
}
