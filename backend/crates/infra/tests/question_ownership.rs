//! Regression tests for the first correctness slice of the hardening plan:
//! `POST /api/questions/{id}/answer` must only act on questions the
//! authenticated learner owns, and must not touch review state otherwise.
//!
//! Exercised through `QuestionService` with the real Postgres repositories,
//! so the check cannot be bypassed at the service boundary.

use app::dtos::question::AnswerRequest;
use app::errors::AppError;
use app::services::question::QuestionService;
use domain::errors::DomainError;
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
        .create(concept.id, "P(A|B) = P(B|A)P(A)/P(B)", "")
        .await
        .unwrap();
    let question = PgQuestionRepository::new(pool.clone())
        .create(ru.id, "recall", "State Bayes' theorem", None, "P(A|B) = P(B|A)P(A)/P(B)", "")
        .await
        .unwrap();

    Fixture { owner, other, ru: ru.id, question: question.id }
}

fn service(pool: &PgPool) -> QuestionService<PgQuestionRepository, PgRuRepository> {
    QuestionService::new(
        PgQuestionRepository::new(pool.clone()),
        PgRuRepository::new(pool.clone()),
    )
}

fn answer(text: &str) -> AnswerRequest {
    AnswerRequest { answer: text.to_string() }
}

async fn review_state(pool: &PgPool, ru: Uuid) -> (i32, String) {
    let r = PgRuRepository::new(pool.clone()).find_by_id(ru).await.unwrap();
    (r.reinforcement_count, r.state.to_string())
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_can_answer_and_review_state_advances(pool: PgPool) {
    let f = seed(&pool).await;

    let res = service(&pool)
        .submit_answer(f.owner, f.question, answer("P(A|B) = P(B|A)P(A)/P(B)"))
        .await
        .expect("owner answers");

    assert!(res.is_correct);
    assert_eq!(review_state(&pool, f.ru).await.0, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn other_learner_is_refused_and_review_state_is_untouched(pool: PgPool) {
    let f = seed(&pool).await;
    let before = review_state(&pool, f.ru).await;

    let res = service(&pool)
        .submit_answer(f.other, f.question, answer("anything"))
        .await;

    assert!(
        matches!(res, Err(AppError::Domain(DomainError::NotFound(_)))),
        "expected NotFound, got {res:?}"
    );
    assert_eq!(review_state(&pool, f.ru).await, before);
}

#[sqlx::test(migrations = "../../migrations")]
async fn nonexistent_question_is_refused(pool: PgPool) {
    let f = seed(&pool).await;

    let res = service(&pool)
        .submit_answer(f.owner, Uuid::new_v4(), answer("anything"))
        .await;

    assert!(matches!(res, Err(AppError::Domain(DomainError::NotFound(_)))), "got {res:?}");
}

#[sqlx::test(migrations = "../../migrations")]
async fn repository_lookup_is_scoped_by_owner(pool: PgPool) {
    let f = seed(&pool).await;
    let repo = PgQuestionRepository::new(pool.clone());

    assert!(repo.find_owned(f.question, f.owner).await.is_ok());
    assert!(matches!(
        repo.find_owned(f.question, f.other).await,
        Err(DomainError::NotFound(_))
    ));
}
