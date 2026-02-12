use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::Concept;

use super::reinforcement_unit::RuResponse;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateConceptRequest {
    #[validate(length(min = 1, max = 200))]
    pub name:        String,
    pub description: Option<String>,
    pub topic_id:    Option<String>,
    pub parent_id:   Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConceptRequest {
    #[validate(length(min = 1, max = 200))]
    pub name:        Option<String>,
    pub description: Option<String>,
    /// `Some(Some(id))` → set; `Some(None)` → unset; `None` → no change
    pub topic_id:    Option<Option<String>>,
    pub parent_id:   Option<Option<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptResponse {
    pub id:                  String,
    pub name:                String,
    pub description:         String,
    pub topic_id:            Option<String>,
    pub parent_id:           Option<String>,
    pub reinforcement_units: Vec<RuResponse>,
    pub created_at:          String,
}

impl From<Concept> for ConceptResponse {
    fn from(c: Concept) -> Self {
        Self {
            id:                  c.id.to_string(),
            name:                c.name,
            description:         c.description,
            topic_id:            c.topic_id.map(|id| id.to_string()),
            parent_id:           c.parent_id.map(|id| id.to_string()),
            reinforcement_units: c.reinforcement_units.into_iter().map(RuResponse::from).collect(),
            created_at:          c.created_at.to_rfc3339(),
        }
    }
}
