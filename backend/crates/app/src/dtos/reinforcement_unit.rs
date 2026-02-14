use serde::{Deserialize, Serialize};

use domain::entities::ReinforcementUnit;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuRequest {
    pub concept_id: String,
    pub items: Vec<CreateRuItem>,
    pub source_resource_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRuItem {
    pub claim: String,
    #[serde(default)]
    pub context: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuRequest {
    pub state:                Option<String>,
    pub stability_score:      Option<f64>,
    pub reinforcement_count:  Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuResponse {
    pub id:                  String,
    pub concept_id:          String,
    pub claim:               String,
    pub context:             String,
    pub claim_id:            Option<String>,
    pub dependency_cost:     f64,
    pub state:               String,
    pub stability_score:     f64,
    pub reinforcement_count: i32,
    pub last_reinforced:     Option<String>,
    pub ease_factor:         f64,
    pub interval_days:       i32,
    pub due_at:              String,
    pub dependencies:        Vec<String>,
    pub source_resource_id:  Option<String>,
    pub created_at:          String,
}

impl From<ReinforcementUnit> for RuResponse {
    fn from(r: ReinforcementUnit) -> Self {
        Self {
            id:                  r.id.to_string(),
            concept_id:          r.concept_id.to_string(),
            claim:               r.claim,
            context:             r.context,
            claim_id:            r.claim_id.map(|id| id.to_string()),
            dependency_cost:     r.dependency_cost,
            state:               r.state.to_string(),
            stability_score:     r.stability_score,
            reinforcement_count: r.reinforcement_count,
            last_reinforced:     r.last_reinforced.map(|t| t.to_rfc3339()),
            ease_factor:         r.ease_factor,
            interval_days:       r.interval_days,
            due_at:              r.due_at.to_rfc3339(),
            dependencies:        r.dependencies.into_iter().map(|id| id.to_string()).collect(),
            source_resource_id:  r.source_resource_id.map(|id| id.to_string()),
            created_at:          r.created_at.to_rfc3339(),
        }
    }
}
