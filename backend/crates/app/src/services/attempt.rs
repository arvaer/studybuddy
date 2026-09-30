use uuid::Uuid;

use domain::learning::RecordAttempt;
use domain::repository_traits::AttemptRepository;

use crate::dtos::attempt::{AttemptReceiptResponse, RecordAttemptRequest};
use crate::errors::AppError;

pub struct AttemptService<R: AttemptRepository> {
    repo: R,
}

impl<R: AttemptRepository> AttemptService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Record one attempt for `user_id`. Ownership, assessment and the
    /// write all happen inside the repository's transaction.
    pub async fn record(&self, user_id: Uuid, req: RecordAttemptRequest) -> Result<AttemptReceiptResponse, AppError> {
        if req.response.is_null() {
            return Err(AppError::Validation("response is required".into()));
        }
        let receipt = self
            .repo
            .record(RecordAttempt {
                user_id,
                activity_revision_id: req.activity_revision_id,
                response: req.response,
                assistance: req.assistance,
            })
            .await?;
        Ok(receipt.into())
    }

    pub async fn get(&self, user_id: Uuid, attempt_id: Uuid) -> Result<AttemptReceiptResponse, AppError> {
        Ok(self.repo.find_receipt(attempt_id, user_id).await?.into())
    }
}
