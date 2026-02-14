use uuid::Uuid;

use domain::repository_traits::{QuizSessionRepository, ReinforcementUnitRepository};

use crate::dtos::quiz::{
    CreateQuizSessionRequest, QuizAnswerResponse, QuizCompleteResponse, QuizSessionResponse,
    SubmitAnswerRequest,
};
use crate::errors::AppError;

pub struct QuizService<Q: QuizSessionRepository, R: ReinforcementUnitRepository> {
    session_repo: Q,
    #[allow(dead_code)]
    ru_repo:      R,  // reserved for SRS due-card queries
}

impl<Q: QuizSessionRepository, R: ReinforcementUnitRepository> QuizService<Q, R> {
    pub fn new(session_repo: Q, ru_repo: R) -> Self {
        Self { session_repo, ru_repo }
    }

    pub async fn create_session(
        &self,
        user_id: Uuid,
        req: CreateQuizSessionRequest,
    ) -> Result<QuizSessionResponse, AppError> {
        let config = serde_json::to_value(&req)
            .map_err(|e| AppError::Unexpected(e.to_string()))?;

        let session = self.session_repo.create(user_id, config).await?;
        Ok(QuizSessionResponse::from(session))
    }

    pub async fn get_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<QuizSessionResponse, AppError> {
        let session = self.session_repo.find_by_id(session_id, user_id).await?;
        let answers = self.session_repo.list_answers(session_id).await?;

        let mut resp = QuizSessionResponse::from(session);
        resp.answers = answers.into_iter().map(QuizAnswerResponse::from).collect();
        Ok(resp)
    }

    pub async fn submit_answer(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        req: SubmitAnswerRequest,
    ) -> Result<QuizAnswerResponse, AppError> {
        // Verify session ownership
        let _ = self.session_repo.find_by_id(session_id, user_id).await?;

        let question_id = Uuid::parse_str(&req.question_id)
            .map_err(|_| AppError::Validation("invalid question_id".into()))?;

        // NOTE: correctness evaluation happens in QuestionService;
        // here we just record the answer as-is (is_correct set by caller or defaulting false).
        // For production, hook into QuestionService.submit_answer and pass result here.
        let answer = self
            .session_repo
            .record_answer(session_id, question_id, &req.answer, false)
            .await?;

        Ok(QuizAnswerResponse::from(answer))
    }

    pub async fn complete_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<QuizCompleteResponse, AppError> {
        let answers = self.session_repo.list_answers(session_id).await?;
        let total   = answers.len() as i32;
        let correct = answers.iter().filter(|a| a.is_correct).count() as i32;

        let session = self
            .session_repo
            .complete(session_id, user_id, correct, total)
            .await?;

        let percentage = if total > 0 {
            correct as f64 / total as f64 * 100.0
        } else {
            0.0
        };

        Ok(QuizCompleteResponse {
            session_id: session.id.to_string(),
            score:      correct,
            total,
            percentage,
        })
    }
}
