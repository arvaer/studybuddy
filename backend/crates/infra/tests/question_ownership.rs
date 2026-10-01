//! `GET /api/questions` must only return questions the authenticated learner
//! owns through RU → concept. The grade-and-mutate answer path was deleted in
//! #15; answering is `POST /api/attempts` (see `record_attempt.rs`).
//!
//! Exercised through `QuestionService` with the real Postgres repository,
//! so the check cannot be bypassed at the service boundary.

use app::services::question::QuestionService;
use domain::repository_traits::{
    ConceptRepository, QuestionRepository, ReinforcementUnitRepository, UserRepository,
};
use infra::repositories::concept::PgConceptRepository;
use infra::repositories::question::PgQuestionRepository;
use infra::repositories::reinforcement_unit::PgRuRepository;
use infra::repositories::user::PgUserRepository;
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    owner:    Uuid,
    other:    Uuid,
    ru:       Uuid,
    question: Uuid,
}

/// Learner A owns concept → RU → question. Learner B owns nothing.
async fn seed(pool: &PgPool) -> Fixture {
    let users = PgUserRepository::new(pool.clone());
    let owner = users.create("a@example.test", "pw", "A").await.unwrap().id;
    let other = users.create("b@example.test", "pw", "B").await.unwrap().id;

    let concept = PgConceptRepository::new(pool.clone())
        .create(owner, None, None, "Bayes", "")
        .await
        .unwrap();
    let ru = PgRuRepository::new(pool.clone())
        .create(owner, concept.id, "P(A|B) = P(B|A)P(A)/P(B)", "")
        .await
        .unwrap();
    let question = PgQuestionRepository::new(pool.clone())
        .create(ru.id, "recall", "State Bayes' theorem", None, "P(A|B) = P(B|A)P(A)/P(B)", "")
        .await
        .unwrap();

    Fixture { owner, other, ru: ru.id, question: question.id }
}

fn service(pool: &PgPool) -> QuestionService<PgQuestionRepository> {
    QuestionService::new(PgQuestionRepository::new(pool.clone()))
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_lists_their_question(pool: PgPool) {
    let f = seed(&pool).await;

    let listed = service(&pool).list(f.owner, None, None, None, None).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, f.question.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn other_learner_sees_nothing_even_through_linked_ids(pool: PgPool) {
    let f = seed(&pool).await;
    let svc = service(&pool);

    // Naming the owner's RU id does not widen the result past the predicate.
    assert!(svc.list(f.other, Some(f.ru), None, None, None).await.unwrap().is_empty());
    assert!(svc.list(f.other, None, None, None, None).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn no_answer_route_mutates_review_state(pool: PgPool) {
    // The RU's review state is only reachable through RuService now; listing
    // questions never touches it.
    let f = seed(&pool).await;
    let before = PgRuRepository::new(pool.clone()).find_by_id(f.ru, f.owner).await.unwrap();

    let _ = service(&pool).list(f.owner, None, None, None, None).await.unwrap();

    let after = PgRuRepository::new(pool.clone()).find_by_id(f.ru, f.owner).await.unwrap();
    assert_eq!(after.reinforcement_count, before.reinforcement_count);
    assert_eq!(after.state, before.state);
}
