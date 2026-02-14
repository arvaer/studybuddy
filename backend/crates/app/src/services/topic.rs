use uuid::Uuid;

use domain::repository_traits::TopicRepository;

use crate::dtos::topic::{CreateTopicRequest, TopicResponse, UpdateTopicRequest};
use crate::errors::AppError;

pub struct TopicService<R: TopicRepository> {
    repo: R,
}

impl<R: TopicRepository> TopicService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<TopicResponse>, AppError> {
        let topics = self.repo.list_by_user(user_id).await?;
        Ok(topics.into_iter().map(TopicResponse::from).collect())
    }

    pub async fn get(&self, id: Uuid, user_id: Uuid) -> Result<TopicResponse, AppError> {
        let topic = self.repo.find_by_id(id, user_id).await?;
        Ok(TopicResponse::from(topic))
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        req: CreateTopicRequest,
    ) -> Result<TopicResponse, AppError> {
        let topic = self
            .repo
            .create(
                user_id,
                &req.name,
                req.description.as_deref().unwrap_or(""),
                req.color.as_deref().unwrap_or("hsl(210, 100%, 50%)"),
            )
            .await?;
        Ok(TopicResponse::from(topic))
    }

    pub async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateTopicRequest,
    ) -> Result<TopicResponse, AppError> {
        let topic = self
            .repo
            .update(
                id,
                user_id,
                req.name.as_deref(),
                req.description.as_deref(),
                req.color.as_deref(),
            )
            .await?;
        Ok(TopicResponse::from(topic))
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.repo.delete(id, user_id).await?;
        Ok(())
    }
}
