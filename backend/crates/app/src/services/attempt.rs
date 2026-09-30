use uuid::Uuid;

use domain::learning::RecordAttempt;
use domain::repository_traits::AttemptRepository;

use crate::dtos::attempt::{AttemptReceiptResponse, RecordAttemptRequest, RecordedAttempt};
use crate::errors::AppError;

/// Bounds the index key; clients send a UUID or similar.
const REQUEST_KEY_MAX_LEN: usize = 128;

pub struct AttemptService<R: AttemptRepository> {
    repo: R,
}

impl<R: AttemptRepository> AttemptService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Record one attempt for `user_id`, or replay the receipt an earlier
    /// submission with the same request key produced. Ownership, the
    /// idempotency check, assessment and the write all happen inside the
    /// repository's transaction.
    pub async fn record(&self, user_id: Uuid, req: RecordAttemptRequest) -> Result<RecordedAttempt, AppError> {
        let key = req.request_key.trim();
        if key.is_empty() || key.len() > REQUEST_KEY_MAX_LEN {
            return Err(AppError::Validation(format!(
                "requestKey must be 1 to {REQUEST_KEY_MAX_LEN} characters"
            )));
        }
        if req.response.is_null() {
            return Err(AppError::Validation("response is required".into()));
        }
        let recorded = self
            .repo
            .record(RecordAttempt {
                user_id,
                request_key: key.to_string(),
                activity_revision_id: req.activity_revision_id,
                response: req.response,
                assistance: req.assistance,
            })
            .await?;
        Ok(RecordedAttempt { receipt: recorded.receipt.into(), replayed: recorded.replayed })
    }

    pub async fn get(&self, user_id: Uuid, attempt_id: Uuid) -> Result<AttemptReceiptResponse, AppError> {
        Ok(self.repo.find_receipt(attempt_id, user_id).await?.into())
    }
}
