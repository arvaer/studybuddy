use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use domain::learning::{ActivityRevision, ActivityWithRevision, RevisionContent};

/// The content of one revision as an author supplies it (#41). `answerKey`
/// and `rubric` are write-only: read responses never return them, so a
/// learner's client cannot see the key of the item it is answering.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionContentRequest {
    pub prompt:             String,
    pub options:            Option<Vec<String>>,
    pub answer_key:         Option<Value>,
    pub rubric:             Option<String>,
    pub source_resource_id: Option<Uuid>,
    /// Where in the source the item comes from; page/offsets today.
    pub source_location:    Option<Value>,
}

impl From<RevisionContentRequest> for RevisionContent {
    fn from(r: RevisionContentRequest) -> Self {
        Self {
            prompt:             r.prompt,
            options:            r.options,
            answer_key:         r.answer_key,
            rubric:             r.rubric,
            source_resource_id: r.source_resource_id,
            source_location:    r.source_location,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateActivityRequest {
    /// `recall`, `explain`, `apply` or `diagnose`.
    pub kind:       String,
    pub concept_id: Option<Uuid>,
    pub revision:   RevisionContentRequest,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RevisionResponse {
    pub id:                 String,
    pub activity_id:        String,
    pub revision:           i32,
    pub prompt:             String,
    pub options:            Option<Vec<String>>,
    /// Whether a deterministic assessment applies; the key itself is withheld.
    pub has_answer_key:     bool,
    pub source_resource_id: Option<String>,
    pub source_location:    Option<Value>,
    pub created_at:         String,
}

impl From<ActivityRevision> for RevisionResponse {
    fn from(r: ActivityRevision) -> Self {
        Self {
            id:                 r.id.to_string(),
            activity_id:        r.activity_id.to_string(),
            revision:           r.revision,
            prompt:             r.prompt,
            options:            r.options,
            has_answer_key:     r.answer_key.is_some(),
            source_resource_id: r.source_resource_id.map(|u| u.to_string()),
            source_location:    r.source_location,
            created_at:         r.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityResponse {
    pub id:         String,
    pub kind:       String,
    pub concept_id: Option<String>,
    pub created_at: String,
    /// The revision a learner selecting this activity now would answer.
    pub current:    RevisionResponse,
}

impl From<ActivityWithRevision> for ActivityResponse {
    fn from(a: ActivityWithRevision) -> Self {
        Self {
            id:         a.activity.id.to_string(),
            kind:       a.activity.kind.as_str().to_string(),
            concept_id: a.activity.concept_id.map(|u| u.to_string()),
            created_at: a.activity.created_at.to_rfc3339(),
            current:    a.current.into(),
        }
    }
}
