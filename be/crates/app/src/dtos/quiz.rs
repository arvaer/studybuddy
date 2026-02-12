use serde::{Deserialize, Serialize};

use domain::entities::{QuizAnswer, QuizSession};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SrsSettings {
    pub new_cards_per_day:    Option<i32>,
    pub reviews_per_day:      Option<i32>,
    pub learning_steps:       Option<Vec<i32>>,   // minutes
    pub graduating_interval:  Option<i32>,         // days
    pub easy_bonus:           Option<f64>,
    pub interval_modifier:    Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CramSettings {
    pub include_stable:  Option<bool>,
    pub priority:        Option<String>,  // "due-first" | "random" | "hardest-first" | "newest-first"
    pub shuffle_order:   Option<bool>,
    pub repeat_missed:   Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuizSessionRequest {
    pub study_mode:          String,  // "srs" | "cram"
    pub session_length_type: Option<String>, // "unlimited" | "cards" | "time"
    pub card_limit:          Option<i32>,
    pub time_limit:          Option<i32>,    // minutes
    pub topic_id:            Option<String>,
    pub concept_id:          Option<String>,
    pub srs_settings:        Option<SrsSettings>,
    pub cram_settings:       Option<CramSettings>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    pub answer:      String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuizSessionResponse {
    pub id:              String,
    pub config:          serde_json::Value,
    pub score:           Option<i32>,
    pub total_questions: Option<i32>,
    pub started_at:      String,
    pub completed_at:    Option<String>,
    pub answers:         Vec<QuizAnswerResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuizAnswerResponse {
    pub id:          String,
    pub question_id: String,
    pub answer:      String,
    pub is_correct:  bool,
    pub answered_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuizCompleteResponse {
    pub session_id:  String,
    pub score:       i32,
    pub total:       i32,
    pub percentage:  f64,
}

impl From<QuizSession> for QuizSessionResponse {
    fn from(s: QuizSession) -> Self {
        Self {
            id:              s.id.to_string(),
            config:          s.config,
            score:           s.score,
            total_questions: s.total_questions,
            started_at:      s.started_at.to_rfc3339(),
            completed_at:    s.completed_at.map(|t| t.to_rfc3339()),
            answers:         vec![],
        }
    }
}

impl From<QuizAnswer> for QuizAnswerResponse {
    fn from(a: QuizAnswer) -> Self {
        Self {
            id:          a.id.to_string(),
            question_id: a.question_id.to_string(),
            answer:      a.answer,
            is_correct:  a.is_correct,
            answered_at: a.answered_at.to_rfc3339(),
        }
    }
}
