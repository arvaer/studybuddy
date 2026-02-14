use serde::Serialize;

use domain::entities::ClaimInContext;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResponse {
    pub id:               String,
    pub concept_id:       String,
    pub predicate:        String,
    pub supporting_text:  String,
    pub asset_id:         Option<String>,
    pub source_location:  Option<serde_json::Value>,
    pub introduced_at:    String,
}

impl From<ClaimInContext> for ClaimResponse {
    fn from(c: ClaimInContext) -> Self {
        Self {
            id:               c.id.to_string(),
            concept_id:       c.concept_id.to_string(),
            predicate:        c.predicate,
            supporting_text:  c.supporting_text,
            asset_id:         c.asset_id.map(|id| id.to_string()),
            source_location:  c.source_location,
            introduced_at:    c.introduced_at.to_rfc3339(),
        }
    }
}
