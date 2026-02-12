use uuid::Uuid;

use domain::repository_traits::{QuestionRepository, ReinforcementUnitRepository};

use crate::dtos::question::{AnswerRequest, AnswerResponse, QuestionResponse};
use crate::dtos::reinforcement_unit::RuResponse;
use crate::errors::AppError;
use crate::services::reinforcement_unit::RuService;

pub struct QuestionService<Q: QuestionRepository, R: ReinforcementUnitRepository> {
    question_repo: Q,
    ru_service:    RuService<R>,
}

impl<Q: QuestionRepository, R: ReinforcementUnitRepository> QuestionService<Q, R> {
    pub fn new(question_repo: Q, ru_repo: R) -> Self {
        Self {
            question_repo,
            ru_service: RuService::new(ru_repo),
        }
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

    /// Submit an answer, evaluate correctness, and advance the RU state via SM-2.
    pub async fn submit_answer(
        &self,
        question_id: Uuid,
        req: AnswerRequest,
    ) -> Result<AnswerResponse, AppError> {
        let question = self.question_repo.find_by_id(question_id).await?;

        let is_correct = question.correct_answer.trim().to_lowercase()
            == req.answer.trim().to_lowercase();

        let ru = self.ru_service.get(question.ru_id).await?;

        // Build update request based on correctness
        use crate::dtos::reinforcement_unit::UpdateRuRequest;
        let (new_stability, new_state, new_count) = advance_ru_state(&ru, is_correct);

        let updated_ru = self
            .ru_service
            .update_after_review(
                question.ru_id,
                UpdateRuRequest {
                    state:               Some(new_state),
                    stability_score:     Some(new_stability),
                    reinforcement_count: Some(new_count),
                },
            )
            .await?;

        Ok(AnswerResponse {
            is_correct,
            explanation: question.explanation,
            updated_ru,
        })
    }
}

/// Determine new state, stability score, and reinforcement count after an answer.
fn advance_ru_state(ru: &RuResponse, correct: bool) -> (f64, String, i32) {
    let count   = ru.reinforcement_count + 1;
    let current = ru.state.as_str();

    if !correct {
        let stability = (ru.stability_score - 0.15).max(0.0);
        let state = if stability < 0.3 { "unstable" } else { current };
        return (stability, state.to_string(), count);
    }

    // Correct answer — advance stability
    let stability = (ru.stability_score + 0.12).min(1.0);

    let state = match current {
        "introduced"  => "reinforced",
        "reinforced"  => {
            if stability >= 0.6 { "stabilizing" } else { "reinforced" }
        }
        "unstable"    => "reinforced",
        "stabilizing" => {
            if stability >= 0.85 { "stable" } else { "stabilizing" }
        }
        "stable"      => "stable",
        other         => other,
    };

    (stability, state.to_string(), count)
}
