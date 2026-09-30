use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use domain::learning::{Assessment, AttemptReceipt};

/// A submission. The revision id names exactly what the learner saw.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordAttemptRequest {
    /// Chosen by the client once per submission (a UUID is fine) and resent
    /// on retry. Same key and payload replays the receipt; same key with a
    /// different payload is refused (#10).
    pub request_key:          String,
    pub activity_revision_id: Uuid,
    /// The answer as submitted: a string for keyed activities, any JSON
    /// otherwise.
    pub response:             Value,
    /// Hints, lookups or explanations used before answering. Recorded as
    /// given; an empty list means unaided.
    #[serde(default)]
    pub assistance:           Vec<Value>,
}

/// What `record` hands back: the receipt plus whether it was replayed, so
/// the route can answer 200 instead of 201.
#[derive(Debug)]
pub struct RecordedAttempt {
    pub receipt:  AttemptReceiptResponse,
    pub replayed: bool,
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
