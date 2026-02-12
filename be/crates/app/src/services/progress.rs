use uuid::Uuid;

use domain::repository_traits::ProgressRepository;

use crate::dtos::progress::ProgressResponse;
use crate::errors::AppError;

pub struct ProgressService<R: ProgressRepository> {
    repo: R,
}

impl<R: ProgressRepository> ProgressService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn get(&self, user_id: Uuid) -> Result<ProgressResponse, AppError> {
        let progress = self.repo.get_progress(user_id).await?;
        Ok(ProgressResponse::from(progress))
    }
}
