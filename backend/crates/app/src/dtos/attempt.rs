use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use domain::learning::{Assessment, AttemptReceipt};

/// A submission. The revision id names exactly what the learner saw.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordAttemptRequest {
    pub activity_revision_id: Uuid,
    /// The answer as submitted: a string for keyed activities, any JSON
    /// otherwise.
    pub response:             Value,
    /// Hints, lookups or explanations used before answering. Recorded as
    /// given; an empty list means unaided.
    #[serde(default)]
    pub assistance:           Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptReceiptResponse {
    pub attempt_id:           String,
    pub activity_revision_id: String,
    pub submitted_at:         String,
    /// `pending`, `correct`, `partial` or `incorrect`.
    pub status:               String,
    pub assessment:           Option<Assessment>,
}

impl From<AttemptReceipt> for AttemptReceiptResponse {
    fn from(r: AttemptReceipt) -> Self {
        Self {
            status:               r.status().to_string(),
            attempt_id:           r.attempt_id.to_string(),
            activity_revision_id: r.activity_revision_id.to_string(),
            submitted_at:         r.submitted_at.to_rfc3339(),
            assessment:           r.assessment,
        }
    }
}
