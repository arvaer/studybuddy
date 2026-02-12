use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::Note;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    #[validate(length(min = 1))]
    pub content:         String,
    pub concept_id:      String,
    pub ru_id:           Option<String>,
    pub anchor_position: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNoteRequest {
    #[validate(length(min = 1))]
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id:               String,
    pub content:          String,
    pub concept_id:       String,
    pub ru_id:            Option<String>,
    pub is_ai_generated:  bool,
    pub anchor_position:  Option<i32>,
    pub created_at:       String,
}

impl From<Note> for NoteResponse {
    fn from(n: Note) -> Self {
        Self {
            id:               n.id.to_string(),
            content:          n.content,
            concept_id:       n.concept_id.to_string(),
            ru_id:            n.ru_id.map(|id| id.to_string()),
            is_ai_generated:  n.is_ai_generated,
            anchor_position:  n.anchor_position,
            created_at:       n.created_at.to_rfc3339(),
        }
    }
}
