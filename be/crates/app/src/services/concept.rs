use uuid::Uuid;

use domain::repository_traits::ConceptRepository;

use crate::dtos::concept::{ConceptResponse, CreateConceptRequest, UpdateConceptRequest};
use crate::errors::AppError;

pub struct ConceptService<R: ConceptRepository> {
    repo: R,
}

impl<R: ConceptRepository> ConceptService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
    ) -> Result<Vec<ConceptResponse>, AppError> {
        let concepts = self.repo.list(user_id, topic_id).await?;
        Ok(concepts.into_iter().map(ConceptResponse::from).collect())
    }

    pub async fn get(&self, id: Uuid, user_id: Uuid) -> Result<ConceptResponse, AppError> {
        let concept = self.repo.find_by_id(id, user_id).await?;
        Ok(ConceptResponse::from(concept))
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        req: CreateConceptRequest,
    ) -> Result<ConceptResponse, AppError> {
        let topic_id = req
            .topic_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| AppError::Validation("invalid topic_id".into()))?;

        let parent_id = req
            .parent_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| AppError::Validation("invalid parent_id".into()))?;

        let concept = self
            .repo
            .create(
                user_id,
                topic_id,
                parent_id,
                &req.name,
                req.description.as_deref().unwrap_or(""),
            )
            .await?;
        Ok(ConceptResponse::from(concept))
    }

    pub async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateConceptRequest,
    ) -> Result<ConceptResponse, AppError> {
        // Parse optional UUID fields
        let topic_id: Option<Option<Uuid>> = match req.topic_id {
            None => None,
            Some(None) => Some(None),
            Some(Some(s)) => Some(Some(
                Uuid::parse_str(&s)
                    .map_err(|_| AppError::Validation("invalid topic_id".into()))?,
            )),
        };

        let parent_id: Option<Option<Uuid>> = match req.parent_id {
            None => None,
            Some(None) => Some(None),
            Some(Some(s)) => Some(Some(
                Uuid::parse_str(&s)
                    .map_err(|_| AppError::Validation("invalid parent_id".into()))?,
            )),
        };

        let concept = self
            .repo
            .update(
                id,
                user_id,
                topic_id,
                parent_id,
                req.name.as_deref(),
                req.description.as_deref(),
            )
            .await?;
        Ok(ConceptResponse::from(concept))
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.repo.delete(id, user_id).await?;
        Ok(())
    }
}
