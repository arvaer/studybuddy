use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::value_objects::{QuestionType, ResourceType, RuState, StudySessionType};

// ─── User ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id:           Uuid,
    pub email:        String,
    pub display_name: String,
    pub avatar:       Option<String>,
    pub created_at:   DateTime<Utc>,
}

// ─── Topic ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id:          Uuid,
    pub user_id:     Uuid,
    pub name:        String,
    pub description: String,
    pub color:       String,
    pub created_at:  DateTime<Utc>,
}

// ─── Concept ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id:                   Uuid,
    pub user_id:              Uuid,
    pub topic_id:             Option<Uuid>,
    pub parent_id:            Option<Uuid>,
    pub name:                 String,
    pub description:          String,
    pub reinforcement_units:  Vec<ReinforcementUnit>,
    pub created_at:           DateTime<Utc>,
}

// ─── ClaimInContext ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimInContext {
    pub id:               Uuid,
    pub concept_id:       Uuid,
    pub predicate:        String,
    pub supporting_text:  String,
    pub asset_id:         Option<Uuid>,
    pub source_location:  Option<serde_json::Value>,
    pub introduced_at:    DateTime<Utc>,
}

// ─── ReinforcementUnit ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReinforcementUnit {
    pub id:                  Uuid,
    pub concept_id:          Uuid,
    pub claim:               String,
    pub context:             String,
    pub claim_id:            Option<Uuid>,
    pub dependency_cost:     f64,
    pub state:               RuState,
    pub stability_score:     f64,
    pub reinforcement_count: i32,
    pub last_reinforced:     Option<DateTime<Utc>>,
    pub ease_factor:         f64,
    pub interval_days:       i32,
    pub due_at:              DateTime<Utc>,
    pub dependencies:        Vec<Uuid>,
    pub source_resource_id:  Option<Uuid>,
    pub created_at:          DateTime<Utc>,
}

// ─── Question ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id:             Uuid,
    pub ru_id:          Uuid,
    pub question_type:  QuestionType,
    pub prompt:         String,
    pub options:        Option<Vec<String>>,
    pub correct_answer: String,
    pub explanation:    String,
    pub created_at:     DateTime<Utc>,
}

// ─── QuizSession ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSession {
    pub id:              Uuid,
    pub user_id:         Uuid,
    pub config:          serde_json::Value,
    pub score:           Option<i32>,
    pub total_questions: Option<i32>,
    pub started_at:      DateTime<Utc>,
    pub completed_at:    Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizAnswer {
    pub id:          Uuid,
    pub session_id:  Uuid,
    pub question_id: Uuid,
    pub answer:      String,
    pub is_correct:  bool,
    pub answered_at: DateTime<Utc>,
}

// ─── StudySession ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySession {
    pub id:           Uuid,
    pub user_id:      Uuid,
    pub title:        String,
    pub session_type: StudySessionType,
    pub concept_ids:  Vec<Uuid>,
    pub progress:     i32,
    pub started_at:   DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ─── Note ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id:               Uuid,
    pub user_id:          Uuid,
    pub concept_id:       Uuid,
    pub ru_id:            Option<Uuid>,
    pub content:          String,
    pub is_ai_generated:  bool,
    pub anchor_position:  Option<i32>,
    pub created_at:       DateTime<Utc>,
}

// ─── Resource ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id:            Uuid,
    pub user_id:       Uuid,
    pub topic_id:      Uuid,
    pub title:         String,
    pub resource_type: ResourceType,
    pub url:           Option<String>,
    pub file_path:     Option<String>,
    pub content_text:  Option<String>,
    pub content_pages: Option<Vec<String>>,
    pub page_count:    i32,
    pub concept_ids:   Vec<Uuid>,
    pub added_at:      DateTime<Utc>,
}

// ─── Event ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id:              Uuid,
    pub event_type:      String,
    pub schema_version:  i32,
    pub emitted_at:      DateTime<Utc>,
    pub producer:        String,
    pub user_id:         Uuid,
    pub aggregate_type:  String,
    pub aggregate_id:    Uuid,
    pub sequence:        i64,
    pub payload:         serde_json::Value,
}

// ─── UserSettings ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub user_id:               Uuid,
    pub reinforcement_prompts: bool,
    pub question_frequency:    i32,
    pub ai_generated_notes:    bool,
    pub study_time_goal:       i32,
    pub daily_questions:       i32,
    pub daily_reminders:       bool,
    pub streak_alerts:         bool,
    pub review_reminders:      bool,
    pub reduce_animations:     bool,
}

// ─── LearnerProgress ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerProgress {
    pub total_concepts:      i64,
    pub stable_concepts:     i64,
    pub needs_reinforcement: i64,
    pub recent_sessions:     i64,
    pub streak_days:         i64,
    pub total_study_time:    i64, // minutes
}
