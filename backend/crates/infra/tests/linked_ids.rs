//! Linked-ID ownership (#29): a write that references another record by id
//! must be refused when that record belongs to a different learner, even
//! though the new row itself would be owned by the caller.

use app::dtos::concept::{CreateConceptRequest, UpdateConceptRequest};
use app::dtos::note::CreateNoteRequest;
use app::dtos::resource::CreateResourceRequest;
use app::dtos::study_session::CreateStudySessionRequest;
use app::errors::AppError;
use app::services::concept::ConceptService;
use app::services::note::NoteService;
use app::services::resource::ResourceService;
use app::services::study_session::StudySessionService;
use domain::errors::DomainError;
use domain::repository_traits::{
    ConceptRepository, ReinforcementUnitRepository, TopicRepository, UserRepository,
};
use infra::repositories::concept::PgConceptRepository;
use infra::repositories::note::PgNoteRepository;
use infra::repositories::reinforcement_unit::PgRuRepository;
use infra::repositories::resource::PgResourceRepository;
use infra::repositories::study_session::PgStudySessionRepository;
use infra::repositories::topic::PgTopicRepository;
use infra::repositories::user::PgUserRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Learner A owns a topic, a concept and an RU. Learner B owns a topic and a concept.
struct Fixture {
    a:         Uuid,
    b:         Uuid,
    a_topic:   Uuid,
    a_concept: Uuid,
    a_ru:      Uuid,
    b_topic:   Uuid,
    b_concept: Uuid,
}

async fn seed(pool: &PgPool) -> Fixture {
    let users = PgUserRepository::new(pool.clone());
    let a = users.create("a@example.test", "pw", "A").await.unwrap().id;
    let b = users.create("b@example.test", "pw", "B").await.unwrap().id;
    let topics = PgTopicRepository::new(pool.clone());
    let a_topic = topics.create(a, "A topic", "", "#000").await.unwrap().id;
    let b_topic = topics.create(b, "B topic", "", "#000").await.unwrap().id;
    let concepts = PgConceptRepository::new(pool.clone());
    let a_concept = concepts.create(a, Some(a_topic), None, "A concept", "").await.unwrap().id;
    let b_concept = concepts.create(b, Some(b_topic), None, "B concept", "").await.unwrap().id;
    let a_ru = PgRuRepository::new(pool.clone())
        .create(a, a_concept, "claim", "")
        .await
        .unwrap()
        .id;
    Fixture { a, b, a_topic, a_concept, a_ru, b_topic, b_concept }
}

fn is_not_found<T: std::fmt::Debug>(r: &Result<T, AppError>) -> bool {
    matches!(r, Err(AppError::Domain(DomainError::NotFound(_))))
}

// ─── notes ───────────────────────────────────────────────────────────────────

fn note_req(concept: Uuid, ru: Option<Uuid>) -> CreateNoteRequest {
    CreateNoteRequest {
        content:         "hello".into(),
        concept_id:      concept.to_string(),
        ru_id:           ru.map(|r| r.to_string()),
        anchor_position: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn note_cannot_link_foreign_concept_or_ru(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = NoteService::new(PgNoteRepository::new(pool.clone()));

    assert!(svc.create(f.a, note_req(f.a_concept, Some(f.a_ru))).await.is_ok());
    assert!(is_not_found(&svc.create(f.b, note_req(f.a_concept, None)).await));
    assert!(is_not_found(&svc.create(f.b, note_req(f.b_concept, Some(f.a_ru))).await));
}

// ─── concepts ────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn concept_cannot_link_foreign_topic_or_parent(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = ConceptService::new(PgConceptRepository::new(pool.clone()));
    let req = |topic: Option<Uuid>, parent: Option<Uuid>| CreateConceptRequest {
        name:        "child".into(),
        description: None,
        topic_id:    topic.map(|t| t.to_string()),
        parent_id:   parent.map(|p| p.to_string()),
    };

    assert!(svc.create(f.b, req(Some(f.b_topic), Some(f.b_concept))).await.is_ok());
    assert!(is_not_found(&svc.create(f.b, req(Some(f.a_topic), None)).await));
    assert!(is_not_found(&svc.create(f.b, req(None, Some(f.a_concept))).await));
}

#[sqlx::test(migrations = "../../migrations")]
async fn concept_update_cannot_relink_to_foreign_records(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = ConceptService::new(PgConceptRepository::new(pool.clone()));
    let relink = |topic: Option<Option<Uuid>>, parent: Option<Option<Uuid>>| UpdateConceptRequest {
        name:        None,
        description: None,
        topic_id:    topic.map(|t| t.map(|t| t.to_string())),
        parent_id:   parent.map(|p| p.map(|p| p.to_string())),
    };

    assert!(is_not_found(&svc.update(f.b_concept, f.b, relink(Some(Some(f.a_topic)), None)).await));
    assert!(is_not_found(&svc.update(f.b_concept, f.b, relink(None, Some(Some(f.a_concept)))).await));
    // Unsetting a link and setting an owned one still work.
    assert!(svc.update(f.b_concept, f.b, relink(Some(None), None)).await.is_ok());
    assert!(svc.update(f.b_concept, f.b, relink(Some(Some(f.b_topic)), None)).await.is_ok());

    let still = svc.get(f.b_concept, f.b).await.unwrap();
    assert_eq!(still.topic_id.as_deref(), Some(f.b_topic.to_string().as_str()));
}

// ─── resources ───────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn resource_cannot_link_foreign_topic_or_concepts(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = ResourceService::new(PgResourceRepository::new(pool.clone()));
    let req = |topic: Uuid, concepts: Vec<Uuid>| CreateResourceRequest {
        title:         "paper".into(),
        resource_type: "article".into(),
        url:           None,
        topic_id:      topic.to_string(),
        concept_ids:   concepts.iter().map(|c| c.to_string()).collect(),
    };

    assert!(svc.create(f.b, req(f.b_topic, vec![f.b_concept])).await.is_ok());
    assert!(is_not_found(&svc.create(f.b, req(f.a_topic, vec![])).await));
    // One foreign id in an otherwise-owned list is enough to refuse.
    assert!(is_not_found(&svc.create(f.b, req(f.b_topic, vec![f.b_concept, f.a_concept])).await));
}

// ─── study sessions ──────────────────────────────────────────────────────────

#[sqlx::test(migrations = "../../migrations")]
async fn study_session_cannot_link_foreign_concepts(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = StudySessionService::new(PgStudySessionRepository::new(pool.clone()));
    let req = |concepts: Vec<Uuid>| CreateStudySessionRequest {
        title:        "session".into(),
        session_type: "reading".into(),
        concept_ids:  concepts.iter().map(|c| c.to_string()).collect(),
    };

    assert!(svc.create(f.b, req(vec![f.b_concept])).await.is_ok());
    assert!(is_not_found(&svc.create(f.b, req(vec![f.a_concept])).await));
}
