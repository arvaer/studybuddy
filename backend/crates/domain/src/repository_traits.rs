use uuid::Uuid;

use crate::entities::{
    ClaimInContext, Concept, Event, LearnerProgress, Note, Question, QuizAnswer, QuizSession,
    ReinforcementUnit, Resource, StudySession, Topic, User, UserSettings,
};
use crate::errors::DomainError;

// ─── UserRepository ──────────────────────────────────────────────────────────

pub trait UserRepository: Send + Sync {
    fn create(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> impl std::future::Future<Output = Result<User, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<User, DomainError>> + Send;

    fn find_by_email(
        &self,
        email: &str,
    ) -> impl std::future::Future<Output = Result<User, DomainError>> + Send;

    /// Returns the user if the password is correct.
    fn verify_password(
        &self,
        email: &str,
        password: &str,
    ) -> impl std::future::Future<Output = Result<User, DomainError>> + Send;

    fn store_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    fn validate_refresh_token(
        &self,
        token_hash: &str,
    ) -> impl std::future::Future<Output = Result<Uuid, DomainError>> + Send;

    fn revoke_refresh_token(
        &self,
        token_hash: &str,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    fn revoke_all_refresh_tokens(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;
}

// ─── TopicRepository ─────────────────────────────────────────────────────────

pub trait TopicRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        name: &str,
        description: &str,
        color: &str,
    ) -> impl std::future::Future<Output = Result<Topic, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Topic, DomainError>> + Send;

    fn list_by_user(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<Topic>, DomainError>> + Send;

    fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        color: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Topic, DomainError>> + Send;

    fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;
}

// ─── ConceptRepository ───────────────────────────────────────────────────────

pub trait ConceptRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
        parent_id: Option<Uuid>,
        name: &str,
        description: &str,
    ) -> impl std::future::Future<Output = Result<Concept, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Concept, DomainError>> + Send;

    fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
    ) -> impl std::future::Future<Output = Result<Vec<Concept>, DomainError>> + Send;

    fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        topic_id: Option<Option<Uuid>>,
        parent_id: Option<Option<Uuid>>,
        name: Option<&str>,
        description: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Concept, DomainError>> + Send;

    fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;
}

// ─── ReinforcementUnitRepository ─────────────────────────────────────────────

pub trait ReinforcementUnitRepository: Send + Sync {
    fn create(
        &self,
        concept_id: Uuid,
        claim: &str,
        context: &str,
    ) -> impl std::future::Future<Output = Result<ReinforcementUnit, DomainError>> + Send;

    fn create_with_source(
        &self,
        concept_id: Uuid,
        claim: &str,
        context: &str,
        source_resource_id: Option<Uuid>,
        claim_id: Option<Uuid>,
    ) -> impl std::future::Future<Output = Result<ReinforcementUnit, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<ReinforcementUnit, DomainError>> + Send;

    fn list(
        &self,
        concept_id: Option<Uuid>,
        state: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<ReinforcementUnit>, DomainError>> + Send;

    fn update_after_review(
        &self,
        id: Uuid,
        state: &str,
        stability_score: f64,
        reinforcement_count: i32,
        ease_factor: f64,
        interval_days: i32,
        due_at: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<Output = Result<ReinforcementUnit, DomainError>> + Send;

    fn list_due(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> impl std::future::Future<Output = Result<Vec<ReinforcementUnit>, DomainError>> + Send;
}

// ─── QuestionRepository ──────────────────────────────────────────────────────

pub trait QuestionRepository: Send + Sync {
    fn create(
        &self,
        ru_id: Uuid,
        question_type: &str,
        prompt: &str,
        options: Option<Vec<String>>,
        correct_answer: &str,
        explanation: &str,
    ) -> impl std::future::Future<Output = Result<Question, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Question, DomainError>> + Send;

    fn list(
        &self,
        ru_id: Option<Uuid>,
        question_type: Option<&str>,
        concept_id: Option<Uuid>,
        topic_id: Option<Uuid>,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<Question>, DomainError>> + Send;
}

// ─── QuizSessionRepository ───────────────────────────────────────────────────

pub trait QuizSessionRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        config: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<QuizSession, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<QuizSession, DomainError>> + Send;

    fn record_answer(
        &self,
        session_id: Uuid,
        question_id: Uuid,
        answer: &str,
        is_correct: bool,
    ) -> impl std::future::Future<Output = Result<QuizAnswer, DomainError>> + Send;

    fn complete(
        &self,
        id: Uuid,
        user_id: Uuid,
        score: i32,
        total: i32,
    ) -> impl std::future::Future<Output = Result<QuizSession, DomainError>> + Send;

    fn list_answers(
        &self,
        session_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<QuizAnswer>, DomainError>> + Send;
}

// ─── StudySessionRepository ──────────────────────────────────────────────────

pub trait StudySessionRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        title: &str,
        session_type: &str,
        concept_ids: &[Uuid],
    ) -> impl std::future::Future<Output = Result<StudySession, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<StudySession, DomainError>> + Send;

    fn list(
        &self,
        user_id: Uuid,
        completed: Option<bool>,
    ) -> impl std::future::Future<Output = Result<Vec<StudySession>, DomainError>> + Send;

    fn update_progress(
        &self,
        id: Uuid,
        user_id: Uuid,
        progress: i32,
    ) -> impl std::future::Future<Output = Result<StudySession, DomainError>> + Send;

    fn complete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<StudySession, DomainError>> + Send;
}

// ─── NoteRepository ──────────────────────────────────────────────────────────

pub trait NoteRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        concept_id: Uuid,
        ru_id: Option<Uuid>,
        content: &str,
        is_ai_generated: bool,
        anchor_position: Option<i32>,
    ) -> impl std::future::Future<Output = Result<Note, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Note, DomainError>> + Send;

    fn list(
        &self,
        user_id: Uuid,
        concept_id: Option<Uuid>,
        ru_id: Option<Uuid>,
    ) -> impl std::future::Future<Output = Result<Vec<Note>, DomainError>> + Send;

    fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> impl std::future::Future<Output = Result<Note, DomainError>> + Send;

    fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;
}

// ─── ResourceRepository ──────────────────────────────────────────────────────

pub trait ResourceRepository: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        title: &str,
        resource_type: &str,
        url: Option<&str>,
        concept_ids: &[Uuid],
    ) -> impl std::future::Future<Output = Result<Resource, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Resource, DomainError>> + Send;

    fn list(
        &self,
        user_id: Uuid,
        topic_id: Option<Uuid>,
        concept_id: Option<Uuid>,
        resource_type: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<Resource>, DomainError>> + Send;

    fn create_uploaded(
        &self,
        user_id: Uuid,
        topic_id: Uuid,
        title: &str,
        resource_type: &str,
        file_path: &str,
        content_text: &str,
        content_pages: &[String],
        concept_ids: &[Uuid],
    ) -> impl std::future::Future<Output = Result<Resource, DomainError>> + Send;

    fn get_content_text(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Option<String>, DomainError>> + Send;

    fn get_pages(
        &self,
        id: Uuid,
        user_id: Uuid,
        page_start: usize,
        page_end: usize,
    ) -> impl std::future::Future<Output = Result<Vec<String>, DomainError>> + Send;

    fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;
}

// ─── ClaimRepository ────────────────────────────────────────────────────────

pub trait ClaimRepository: Send + Sync {
    fn create(
        &self,
        concept_id: Uuid,
        predicate: &str,
        supporting_text: &str,
        asset_id: Option<Uuid>,
        source_location: Option<serde_json::Value>,
    ) -> impl std::future::Future<Output = Result<ClaimInContext, DomainError>> + Send;

    fn find_by_id(
        &self,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<ClaimInContext, DomainError>> + Send;

    fn list_by_concept(
        &self,
        concept_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<ClaimInContext>, DomainError>> + Send;

    fn list_by_asset(
        &self,
        asset_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<ClaimInContext>, DomainError>> + Send;
}

// ─── EventRepository ────────────────────────────────────────────────────────

pub trait EventRepository: Send + Sync {
    fn append(
        &self,
        event: &Event,
    ) -> impl std::future::Future<Output = Result<Event, DomainError>> + Send;

    fn list_by_aggregate(
        &self,
        aggregate_type: &str,
        aggregate_id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<Event>, DomainError>> + Send;
}

// ─── SettingsRepository ──────────────────────────────────────────────────────

pub trait SettingsRepository: Send + Sync {
    fn get(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<UserSettings, DomainError>> + Send;

    fn upsert(
        &self,
        settings: &UserSettings,
    ) -> impl std::future::Future<Output = Result<UserSettings, DomainError>> + Send;
}

// ─── ProgressRepository ──────────────────────────────────────────────────────

pub trait ProgressRepository: Send + Sync {
    fn get_progress(
        &self,
        user_id: Uuid,
    ) -> impl std::future::Future<Output = Result<LearnerProgress, DomainError>> + Send;
}
