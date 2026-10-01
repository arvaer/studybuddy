use uuid::Uuid;

use domain::repository_traits::QuestionRepository;

use crate::dtos::question::QuestionResponse;
use crate::errors::AppError;

/// Read-only access to a learner's questions. Answering moved to
/// `POST /api/attempts` (persisted attempts, #8); the old grade-and-mutate
/// path was deleted in #15.
pub struct QuestionService<Q: QuestionRepository> {
    question_repo: Q,
}

impl<Q: QuestionRepository> QuestionService<Q> {
    pub fn new(question_repo: Q) -> Self {
        Self { question_repo }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        ru_id: Option<Uuid>,
        concept_id: Option<Uuid>,
        topic_id: Option<Uuid>,
        question_type: Option<&str>,
    ) -> Result<Vec<QuestionResponse>, AppError> {
        let questions = self
            .question_repo
            .list(ru_id, question_type, concept_id, topic_id, user_id)
            .await?;
        Ok(questions.into_iter().map(QuestionResponse::from).collect())
    }
}
