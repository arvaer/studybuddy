use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::StudySession;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateStudySessionRequest {
    #[validate(length(min = 1, max = 300))]
    pub title:       String,
    #[serde(rename = "type")]
    pub session_type: String,
    pub concept_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    pub progress: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySessionResponse {
    pub id:           String,
    pub title:        String,
    #[serde(rename = "type")]
    pub session_type: String,
    pub concept_ids:  Vec<String>,
    pub progress:     i32,
    pub started_at:   String,
    pub completed_at: Option<String>,
}

impl From<StudySession> for StudySessionResponse {
    fn from(s: StudySession) -> Self {
        Self {
            id:           s.id.to_string(),
            title:        s.title,
            session_type: s.session_type.to_string(),
            concept_ids:  s.concept_ids.iter().map(|id| id.to_string()).collect(),
            progress:     s.progress,
            started_at:   s.started_at.to_rfc3339(),
            completed_at: s.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}
