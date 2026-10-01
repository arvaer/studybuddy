//! Declared DTO constraints are enforced at every service entry point (#36):
//! one case per request family proves the constraint trips as
//! `AppError::Validation` and nothing is written.

use app::dtos::concept::CreateConceptRequest;
use app::dtos::note::{CreateNoteRequest, UpdateNoteRequest};
use app::dtos::resource::CreateResourceRequest;
use app::dtos::study_session::CreateStudySessionRequest;
use app::dtos::topic::{CreateTopicRequest, UpdateTopicRequest};
use app::errors::AppError;
use app::services::concept::ConceptService;
use app::services::note::NoteService;
use app::services::resource::ResourceService;
use app::services::study_session::StudySessionService;
use app::services::topic::TopicService;
use infra::repositories::concept::PgConceptRepository;
use infra::repositories::note::PgNoteRepository;
use infra::repositories::resource::PgResourceRepository;
use infra::repositories::study_session::PgStudySessionRepository;
use infra::repositories::topic::PgTopicRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ('v@example.test', 'x', 'L') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

fn is_validation<T: std::fmt::Debug>(r: Result<T, AppError>) -> bool {
    matches!(r, Err(AppError::Validation(_)))
}

#[sqlx::test(migrations = "../../migrations")]
async fn topic_name_bounds_are_enforced_on_create_and_update(pool: PgPool) {
    let user = learner(&pool).await;
    let svc = TopicService::new(PgTopicRepository::new(pool.clone()));

    let empty = CreateTopicRequest { name: "".into(), description: None, color: None };
    assert!(is_validation(svc.create(user, empty).await));
    let long = CreateTopicRequest { name: "x".repeat(201), description: None, color: None };
    assert!(is_validation(svc.create(user, long).await));
    assert_eq!(count(&pool, "topics").await, 0);

    let ok = svc.create(user, CreateTopicRequest { name: "RL".into(), description: None, color: None }).await.unwrap();
    let id: Uuid = ok.id.parse().unwrap();
    let blank = UpdateTopicRequest { name: Some("".into()), description: None, color: None };
    assert!(is_validation(svc.update(id, user, blank).await));
    assert_eq!(svc.get(id, user).await.unwrap().name, "RL");
}

#[sqlx::test(migrations = "../../migrations")]
async fn concept_name_is_required(pool: PgPool) {
    let user = learner(&pool).await;
    let svc = ConceptService::new(PgConceptRepository::new(pool.clone()));

    let req = CreateConceptRequest { name: "".into(), description: None, topic_id: None, parent_id: None };
    assert!(is_validation(svc.create(user, req).await));
    assert_eq!(count(&pool, "concepts").await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn note_content_is_required_on_create_and_update(pool: PgPool) {
    let user = learner(&pool).await;
    let svc = NoteService::new(PgNoteRepository::new(pool.clone()));

    let req = CreateNoteRequest { content: "".into(), concept_id: Uuid::new_v4().to_string(), ru_id: None, anchor_position: None };
    assert!(is_validation(svc.create(user, req).await));
    assert!(is_validation(svc.update(Uuid::new_v4(), user, UpdateNoteRequest { content: "".into() }).await));
    assert_eq!(count(&pool, "notes").await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn resource_title_bounds_are_enforced(pool: PgPool) {
    let user = learner(&pool).await;
    let svc = ResourceService::new(PgResourceRepository::new(pool.clone()));

    let req = CreateResourceRequest {
        title: "x".repeat(301),
        resource_type: "article".into(),
        url: None,
        topic_id: Uuid::new_v4().to_string(),
        concept_ids: vec![],
    };
    assert!(is_validation(svc.create(user, req).await));
    assert_eq!(count(&pool, "resources").await, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn study_session_title_is_required(pool: PgPool) {
    let user = learner(&pool).await;
    let svc = StudySessionService::new(PgStudySessionRepository::new(pool.clone()));

    let req = CreateStudySessionRequest { title: "".into(), session_type: "review".into(), concept_ids: vec![] };
    assert!(is_validation(svc.create(user, req).await));
    assert_eq!(count(&pool, "study_sessions").await, 0);
}
