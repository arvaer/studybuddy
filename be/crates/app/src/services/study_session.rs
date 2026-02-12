use uuid::Uuid;

use domain::repository_traits::StudySessionRepository;

use crate::dtos::study_session::{
    CreateStudySessionRequest, StudySessionResponse, UpdateProgressRequest,
};
use crate::errors::AppError;

pub struct StudySessionService<R: StudySessionRepository> {
    repo: R,
}

impl<R: StudySessionRepository> StudySessionService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        completed: Option<bool>,
    ) -> Result<Vec<StudySessionResponse>, AppError> {
        let sessions = self.repo.list(user_id, completed).await?;
        Ok(sessions.into_iter().map(StudySessionResponse::from).collect())
    }

    pub async fn get(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<StudySessionResponse, AppError> {
        let s = self.repo.find_by_id(id, user_id).await?;
        Ok(StudySessionResponse::from(s))
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        req: CreateStudySessionRequest,
    ) -> Result<StudySessionResponse, AppError> {
        let concept_ids = req
            .concept_ids
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::Validation("invalid concept_id in list".into()))?;

        let s = self
            .repo
            .create(user_id, &req.title, &req.session_type, &concept_ids)
            .await?;
        Ok(StudySessionResponse::from(s))
    }

    pub async fn update_progress(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateProgressRequest,
    ) -> Result<StudySessionResponse, AppError> {
        if !(0..=100).contains(&req.progress) {
            return Err(AppError::Validation("progress must be 0–100".into()));
        }
        let s = self.repo.update_progress(id, user_id, req.progress).await?;
        Ok(StudySessionResponse::from(s))
    }

    pub async fn complete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<StudySessionResponse, AppError> {
        let s = self.repo.complete(id, user_id).await?;
        Ok(StudySessionResponse::from(s))
    }
}
