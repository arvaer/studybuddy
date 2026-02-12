use serde::Serialize;

use domain::entities::LearnerProgress;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressResponse {
    pub total_concepts:      i64,
    pub stable_concepts:     i64,
    pub needs_reinforcement: i64,
    pub recent_sessions:     i64,
    pub streak_days:         i64,
    pub total_study_time:    i64,
}

impl From<LearnerProgress> for ProgressResponse {
    fn from(p: LearnerProgress) -> Self {
        Self {
            total_concepts:      p.total_concepts,
            stable_concepts:     p.stable_concepts,
            needs_reinforcement: p.needs_reinforcement,
            recent_sessions:     p.recent_sessions,
            streak_days:         p.streak_days,
            total_study_time:    p.total_study_time,
        }
    }
}
