use serde::{Deserialize, Serialize};

use domain::entities::Question;

#[derive(Debug, Deserialize)]
pub struct AnswerRequest {
    pub answer: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerResponse {
    pub is_correct:  bool,
    pub explanation: String,
    pub updated_ru:  super::reinforcement_unit::RuResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionResponse {
    pub id:             String,
    pub ru_id:          String,
    #[serde(rename = "type")]
    pub question_type:  String,
    pub prompt:         String,
    pub options:        Option<Vec<String>>,
    pub correct_answer: String,
    pub explanation:    String,
    pub created_at:     String,
}

impl From<Question> for QuestionResponse {
    fn from(q: Question) -> Self {
        Self {
            id:             q.id.to_string(),
            ru_id:          q.ru_id.to_string(),
            question_type:  q.question_type.to_string(),
            prompt:         q.prompt,
            options:        q.options,
            correct_answer: q.correct_answer,
            explanation:    q.explanation,
            created_at:     q.created_at.to_rfc3339(),
        }
    }
}
