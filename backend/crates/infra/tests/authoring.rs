//! Authoring activities and revisions (#41): create with revision 1, revise
//! without touching earlier revisions or their attempts, owner-scoped reads,
//! two-learner ownership, content validation, and concurrent revisions.

use app::dtos::activity::{CreateActivityRequest, RevisionContentRequest};
use app::dtos::attempt::RecordAttemptRequest;
use app::errors::AppError;
use app::services::activity::ActivityService;
use app::services::attempt::AttemptService;
use domain::errors::DomainError;
use infra::repositories::activity::PgActivityRepository;
use infra::repositories::attempt::PgAttemptRepository;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn learner(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// A topic, a concept in it and a PDF resource in it, all owned by `user_id`.
async fn concept_and_resource(pool: &PgPool, user_id: Uuid) -> (Uuid, Uuid) {
    let topic: Uuid =
        sqlx::query_scalar("INSERT INTO topics (user_id, name) VALUES ($1, 'RL') RETURNING id")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let concept: Uuid = sqlx::query_scalar(
        "INSERT INTO concepts (user_id, topic_id, name) VALUES ($1, $2, 'Policy') RETURNING id",
    )
    .bind(user_id)
    .bind(topic)
    .fetch_one(pool)
    .await
    .unwrap();
    let resource: Uuid = sqlx::query_scalar(
        "INSERT INTO resources (user_id, topic_id, title, resource_type) VALUES ($1, $2, 'Sutton & Barto', 'pdf') RETURNING id",
    )
    .bind(user_id)
    .bind(topic)
    .fetch_one(pool)
    .await
    .unwrap();
    (concept, resource)
}

fn content(prompt: &str, answer_key: Option<Value>) -> RevisionContentRequest {
    RevisionContentRequest {
        prompt: prompt.into(),
        options: None,
        answer_key,
        rubric: None,
        source_resource_id: None,
        source_location: None,
    }
}

fn create_req(
    kind: &str,
    concept_id: Option<Uuid>,
    revision: RevisionContentRequest,
) -> CreateActivityRequest {
    CreateActivityRequest {
        kind: kind.into(),
        concept_id,
        revision,
    }
}

fn not_found<T>(r: Result<T, AppError>) -> bool {
    matches!(r, Err(AppError::Domain(DomainError::NotFound(_))))
}

fn validation<T>(r: Result<T, AppError>) -> bool {
    matches!(
        r,
        Err(AppError::Validation(_)) | Err(AppError::Domain(DomainError::Validation(_)))
    )
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_writes_the_activity_and_revision_one_together(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let (concept, resource) = concept_and_resource(&pool, u).await;
    let svc = ActivityService::new(PgActivityRepository::new(pool.clone()));

    let created = svc
        .create(
            u,
            create_req(
                "recall",
                Some(concept),
                RevisionContentRequest {
                    source_resource_id: Some(resource),
                    source_location: Some(json!({"page": 58})),
                    ..content(
                        "  What does a policy specify?  ",
                        Some(json!("A mapping from states to actions")),
                    )
                },
            ),
        )
        .await
        .unwrap();

    assert_eq!(created.kind, "recall");
    assert_eq!(created.concept_id, Some(concept.to_string()));
    assert_eq!(created.current.revision, 1);
    assert_eq!(created.current.prompt, "What does a policy specify?");
    assert!(created.current.has_answer_key);
    assert_eq!(
        created.current.source_resource_id,
        Some(resource.to_string())
    );
    assert_eq!(created.current.source_location, Some(json!({"page": 58})));
    assert_eq!(count(&pool, "activities").await, 1);
    assert_eq!(count(&pool, "activity_revisions").await, 1);

    // The key is stored but never read back through the API.
    let key: Value = sqlx::query_scalar("SELECT answer_key FROM activity_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(key, json!("A mapping from states to actions"));
    let listed = svc.list(u).await.unwrap();
    assert_eq!(
        listed,
        vec![svc.get(u, created.id.parse().unwrap()).await.unwrap()]
    );
    assert_eq!(
        svc.get_revision(u, created.current.id.parse().unwrap())
            .await
            .unwrap(),
        created.current
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn revising_leaves_revision_one_and_its_attempt_untouched(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let activities = ActivityService::new(PgActivityRepository::new(pool.clone()));
    let attempts = AttemptService::new(PgAttemptRepository::new(pool.clone()));

    let created = activities
        .create(
            u,
            create_req("recall", None, content("v1 prompt", Some(json!("policy")))),
        )
        .await
        .unwrap();
    let rev1: Uuid = created.current.id.parse().unwrap();
    let receipt = attempts
        .record(
            u,
            RecordAttemptRequest {
                request_key: "k".into(),
                activity_revision_id: rev1,
                response: json!("Policy"),
                assistance: vec![],
            },
        )
        .await
        .unwrap()
        .receipt;
    assert_eq!(
        receipt.status, "pending",
        "free text is the operator's to assess (20f)"
    );

    let rev2 = activities
        .revise(
            u,
            created.id.parse().unwrap(),
            content("v2 prompt", Some(json!("value function"))),
        )
        .await
        .unwrap();
    assert_eq!(rev2.revision, 2);
    assert_eq!(rev2.prompt, "v2 prompt");

    // Revision 1 reads back unchanged, the attempt still points at it, and
    // the activity's current revision is now 2.
    let r1 = activities.get_revision(u, rev1).await.unwrap();
    assert_eq!((r1.revision, r1.prompt.as_str()), (1, "v1 prompt"));
    let again = attempts
        .get(u, receipt.attempt_id.parse().unwrap())
        .await
        .unwrap();
    assert_eq!(
        (again.activity_revision_id, again.status.as_str()),
        (rev1.to_string(), "pending")
    );
    assert_eq!(
        activities
            .get(u, created.id.parse().unwrap())
            .await
            .unwrap()
            .current,
        rev2
    );
    assert_eq!(count(&pool, "activity_revisions").await, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn second_learner_is_refused_everywhere_and_writes_nothing(pool: PgPool) {
    let a = learner(&pool, "a@example.com").await;
    let b = learner(&pool, "b@example.com").await;
    let (concept_a, resource_a) = concept_and_resource(&pool, a).await;
    let svc = ActivityService::new(PgActivityRepository::new(pool.clone()));

    let mine = svc
        .create(
            a,
            create_req(
                "explain",
                None,
                content("Explain the Bellman equation.", None),
            ),
        )
        .await
        .unwrap();
    let activity: Uuid = mine.id.parse().unwrap();
    let revision: Uuid = mine.current.id.parse().unwrap();

    assert!(not_found(svc.get(b, activity).await));
    assert!(not_found(svc.get_revision(b, revision).await));
    assert!(not_found(
        svc.revise(b, activity, content("hijacked", None)).await
    ));
    assert!(svc.list(b).await.unwrap().is_empty());

    // Linking A's concept or A's resource from B's own activity is refused
    // before any row is written.
    assert!(not_found(
        svc.create(b, create_req("recall", Some(concept_a), content("p", None)))
            .await
    ));
    assert!(not_found(
        svc.create(
            b,
            create_req(
                "recall",
                None,
                RevisionContentRequest {
                    source_resource_id: Some(resource_a),
                    ..content("p", None)
                }
            )
        )
        .await
    ));
    assert!(not_found(
        svc.revise(
            b,
            activity,
            RevisionContentRequest {
                source_resource_id: Some(resource_a),
                ..content("p", None)
            }
        )
        .await
    ));

    assert_eq!(count(&pool, "activities").await, 1);
    assert_eq!(count(&pool, "activity_revisions").await, 1);
    assert_eq!(
        svc.get_revision(a, revision).await.unwrap().prompt,
        "Explain the Bellman equation."
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_content_or_kind_is_rejected_before_any_write(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let svc = ActivityService::new(PgActivityRepository::new(pool.clone()));

    assert!(validation(
        svc.create(u, create_req("quiz", None, content("p", None)))
            .await
    ));
    assert!(validation(
        svc.create(u, create_req("recall", None, content("   ", None)))
            .await
    ));
    assert!(validation(
        svc.create(
            u,
            create_req(
                "recall",
                None,
                RevisionContentRequest {
                    options: Some(vec!["Reward".into(), "Value".into()]),
                    ..content("p", Some(json!("Policy")))
                }
            )
        )
        .await
    ));
    assert!(validation(
        svc.create(
            u,
            create_req(
                "recall",
                None,
                RevisionContentRequest {
                    source_location: Some(json!({"page": 1})),
                    ..content("p", None)
                }
            )
        )
        .await
    ));

    let ok = svc
        .create(u, create_req("recall", None, content("p", None)))
        .await
        .unwrap();
    assert!(validation(
        svc.revise(u, ok.id.parse().unwrap(), content("", None))
            .await
    ));

    assert_eq!(count(&pool, "activities").await, 1);
    assert_eq!(count(&pool, "activity_revisions").await, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_revisions_number_distinctly(pool: PgPool) {
    let u = learner(&pool, "a@example.com").await;
    let svc = ActivityService::new(PgActivityRepository::new(pool.clone()));
    let activity: Uuid = svc
        .create(u, create_req("recall", None, content("v1", None)))
        .await
        .unwrap()
        .id
        .parse()
        .unwrap();

    let tasks: Vec<_> = (0..8)
        .map(|i| {
            let svc = ActivityService::new(PgActivityRepository::new(pool.clone()));
            tokio::spawn(async move {
                svc.revise(u, activity, content(&format!("edit {i}"), None))
                    .await
            })
        })
        .collect();

    let mut numbers = Vec::new();
    for t in tasks {
        numbers.push(t.await.unwrap().unwrap().revision);
    }
    numbers.sort_unstable();
    assert_eq!(numbers, (2..=9).collect::<Vec<i32>>());
    assert_eq!(svc.get(u, activity).await.unwrap().current.revision, 9);
}
