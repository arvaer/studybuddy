use serde::{Deserialize, Serialize};

use domain::entities::UserSettings;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    pub reinforcement_prompts: Option<bool>,
    pub question_frequency:    Option<i32>,
    pub ai_generated_notes:    Option<bool>,
    pub study_time_goal:       Option<i32>,
    pub daily_questions:       Option<i32>,
    pub daily_reminders:       Option<bool>,
    pub streak_alerts:         Option<bool>,
    pub review_reminders:      Option<bool>,
    pub reduce_animations:     Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    pub reinforcement_prompts: bool,
    pub question_frequency:    i32,
    pub ai_generated_notes:    bool,
    pub study_time_goal:       i32,
    pub daily_questions:       i32,
    pub daily_reminders:       bool,
    pub streak_alerts:         bool,
    pub review_reminders:      bool,
    pub reduce_animations:     bool,
}

impl From<UserSettings> for SettingsResponse {
    fn from(s: UserSettings) -> Self {
        Self {
            reinforcement_prompts: s.reinforcement_prompts,
            question_frequency:    s.question_frequency,
            ai_generated_notes:    s.ai_generated_notes,
            study_time_goal:       s.study_time_goal,
            daily_questions:       s.daily_questions,
            daily_reminders:       s.daily_reminders,
            streak_alerts:         s.streak_alerts,
            review_reminders:      s.review_reminders,
            reduce_animations:     s.reduce_animations,
        }
    }
}

impl UpdateSettingsRequest {
    /// Merge patch into existing settings.
    pub fn apply_to(self, mut s: UserSettings, user_id: Uuid) -> UserSettings {
        s.user_id = user_id;
        if let Some(v) = self.reinforcement_prompts { s.reinforcement_prompts = v; }
        if let Some(v) = self.question_frequency    { s.question_frequency = v; }
        if let Some(v) = self.ai_generated_notes    { s.ai_generated_notes = v; }
        if let Some(v) = self.study_time_goal       { s.study_time_goal = v; }
        if let Some(v) = self.daily_questions       { s.daily_questions = v; }
        if let Some(v) = self.daily_reminders       { s.daily_reminders = v; }
        if let Some(v) = self.streak_alerts         { s.streak_alerts = v; }
        if let Some(v) = self.review_reminders      { s.review_reminders = v; }
        if let Some(v) = self.reduce_animations     { s.reduce_animations = v; }
        s
    }
}
