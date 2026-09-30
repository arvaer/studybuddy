use uuid::Uuid;

use domain::learning::{ActivityKind, NewActivity, NewRevision};
use domain::repository_traits::ActivityRepository;

use crate::dtos::activity::{ActivityResponse, CreateActivityRequest, RevisionContentRequest, RevisionResponse};
use crate::errors::AppError;

pub struct ActivityService<R: ActivityRepository> {
    repo: R,
}

impl<R: ActivityRepository> ActivityService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Create an activity with revision 1 (#41). Content validation and the
    /// linked-id ownership checks happen in the domain and the repository.
    pub async fn create(&self, user_id: Uuid, req: CreateActivityRequest) -> Result<ActivityResponse, AppError> {
        let kind = ActivityKind::parse(&req.kind)
            .ok_or_else(|| AppError::Validation("kind must be recall, explain, apply or diagnose".into()))?;
        let created = self
            .repo
            .create(NewActivity { user_id, kind, concept_id: req.concept_id, content: req.revision.into() })
            .await?;
        Ok(created.into())
    }

    /// Add the next revision to an owned activity.
    pub async fn revise(
        &self,
        user_id: Uuid,
        activity_id: Uuid,
        req: RevisionContentRequest,
    ) -> Result<RevisionResponse, AppError> {
        let revision = self.repo.revise(NewRevision { user_id, activity_id, content: req.into() }).await?;
        Ok(revision.into())
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<ActivityResponse>, AppError> {
        Ok(self.repo.list(user_id).await?.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, user_id: Uuid, activity_id: Uuid) -> Result<ActivityResponse, AppError> {
        Ok(self.repo.find(activity_id, user_id).await?.into())
    }

    pub async fn get_revision(&self, user_id: Uuid, revision_id: Uuid) -> Result<RevisionResponse, AppError> {
        Ok(self.repo.find_revision(revision_id, user_id).await?.into())
    }
}
