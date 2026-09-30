//! Ownership audit (#4): the reinforcement-unit routes previously discarded
//! `AuthUser` entirely. Every repository method now carries the learner and
//! resolves ownership through `concepts.user_id`, including the linked-ID
//! path where a learner supplies another learner's concept id on create.

use app::dtos::reinforcement_unit::{CreateRuItem, CreateRuRequest, UpdateRuRequest};
use app::errors::AppError;
use app::services::reinforcement_unit::RuService;
use domain::errors::DomainError;
use domain::repository_traits::{ConceptRepository, ReinforcementUnitRepository, UserRepository};
use infra::repositories::concept::PgConceptRepository;
use infra::repositories::reinforcement_unit::PgRuRepository;
use infra::repositories::user::PgUserRepository;
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    owner:   Uuid,
    other:   Uuid,
    concept: Uuid,
    ru:      Uuid,
}

async fn seed(pool: &PgPool) -> Fixture {
    let users = PgUserRepository::new(pool.clone());
    let owner = users.create("a@example.test", "pw", "A").await.unwrap().id;
    let other = users.create("b@example.test", "pw", "B").await.unwrap().id;
    let concept = PgConceptRepository::new(pool.clone())
        .create(owner, None, None, "Bayes", "")
        .await
        .unwrap()
        .id;
    let ru = PgRuRepository::new(pool.clone())
        .create(owner, concept, "claim", "")
        .await
        .unwrap()
        .id;
    Fixture { owner, other, concept, ru }
}

fn svc(pool: &PgPool) -> RuService<PgRuRepository> {
    RuService::new(PgRuRepository::new(pool.clone()))
}

fn is_not_found<T: std::fmt::Debug>(r: &Result<T, AppError>) -> bool {
    matches!(r, Err(AppError::Domain(DomainError::NotFound(_))))
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_without_filter_only_returns_own_rus(pool: PgPool) {
    let f = seed(&pool).await;

    let mine   = svc(&pool).list(f.owner, None, None).await.unwrap();
    let theirs = svc(&pool).list(f.other, None, None).await.unwrap();

    assert_eq!(mine.len(), 1);
    assert!(theirs.is_empty(), "learner B saw learner A's RUs: {theirs:?}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_by_foreign_concept_id_is_empty(pool: PgPool) {
    let f = seed(&pool).await;

    let theirs = svc(&pool).list(f.other, Some(f.concept), None).await.unwrap();

    assert!(theirs.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_is_scoped_by_owner(pool: PgPool) {
    let f = seed(&pool).await;

    assert!(svc(&pool).get(f.ru, f.owner).await.is_ok());
    assert!(is_not_found(&svc(&pool).get(f.ru, f.other).await));
}

#[sqlx::test(migrations = "../../migrations")]
async fn other_learner_cannot_update_review_state(pool: PgPool) {
    let f = seed(&pool).await;
    let req = || UpdateRuRequest {
        state:               Some("stable".into()),
        stability_score:     Some(1.0),
        reinforcement_count: Some(99),
    };

    let res = svc(&pool).update_after_review(f.ru, f.other, req()).await;
    assert!(is_not_found(&res), "got {res:?}");

    let after = svc(&pool).get(f.ru, f.owner).await.unwrap();
    assert_eq!(after.reinforcement_count, 0);
    assert_eq!(after.state, "introduced");

    // The owner still can.
    let updated = svc(&pool).update_after_review(f.ru, f.owner, req()).await.unwrap();
    assert_eq!(updated.reinforcement_count, 99);
}

#[sqlx::test(migrations = "../../migrations")]
async fn cannot_create_ru_under_foreign_concept(pool: PgPool) {
    let f = seed(&pool).await;
    let req = CreateRuRequest {
        concept_id:         f.concept.to_string(),
        items:              vec![CreateRuItem { claim: "planted".into(), context: String::new() }],
        source_resource_id: None,
    };

    let res = svc(&pool).create(f.other, req).await;

    assert!(is_not_found(&res), "got {res:?}");
    let owners_view = svc(&pool).list(f.owner, Some(f.concept), None).await.unwrap();
    assert_eq!(owners_view.len(), 1, "a foreign RU was planted in A's concept");
}
