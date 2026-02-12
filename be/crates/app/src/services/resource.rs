use uuid::Uuid;

use domain::repository_traits::ResourceRepository;

use crate::dtos::resource::{CreateResourceRequest, ResourceResponse};
use crate::errors::AppError;

pub struct ResourceService<R: ResourceRepository> {
    repo: R,
}

impl<R: ResourceRepository> ResourceService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
        concept_id: Option<Uuid>,
        resource_type: Option<&str>,
    ) -> Result<Vec<ResourceResponse>, AppError> {
        let resources = self.repo.list(user_id, topic_id, concept_id, resource_type).await?;
        Ok(resources.into_iter().map(ResourceResponse::from).collect())
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        req: CreateResourceRequest,
    ) -> Result<ResourceResponse, AppError> {
        let topic_id = Uuid::parse_str(&req.topic_id)
            .map_err(|_| AppError::Validation("invalid topic_id".into()))?;

        let concept_ids = req
            .concept_ids
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::Validation("invalid concept_id in list".into()))?;

        let resource = self
            .repo
            .create(
                user_id,
                topic_id,
                &req.title,
                &req.resource_type,
                req.url.as_deref(),
                &concept_ids,
            )
            .await?;
        Ok(ResourceResponse::from(resource))
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.repo.delete(id, user_id).await?;
        Ok(())
    }
}
