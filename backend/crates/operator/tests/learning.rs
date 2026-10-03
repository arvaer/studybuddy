//! 19c: the learning capsule in the repository runs the loop on the owner
//! with real PgStorage. A scripted model presents, reads the attempt,
//! presents a follow-up, finishes; a scripted `learning/present` answers a
//! publication; `learner/wait` has no provider and parks, and the test
//! completes it as 20b will, by observing the attempt. A capsule filled for
//! one workspace is refused by another's environment at admission, before
//! it is even instantiated.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use capsule_corp::sdk::{Answer, Effect, Outcome, Reply, Session};
use common::{url, workspace};
use operator::capsule;
use operator::host::{Install, Record};
use operator::OperatorHost;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

type Calls = Arc<Mutex<Vec<Effect>>>;

const GOAL: &str = "Understand the return and discounting in RL.";
const FIRST: &str =
    r#"(coach/present "What does the return G_t sum?" "the discounted sum of future rewards")"#;
const FOLLOW_UP: &str =
    r#"(coach/present "Why discount at all?" "to bound the sum and prefer sooner reward")"#;
const FINISH: &str = r#"(coach/finish "You can define the return and say why it is discounted.")"#;

/// What the endpoint answers for a publication: the created activity.
fn publication(effect: &Effect) -> Value {
    let payload = effect.payload();
    json!({ "id": format!("activity-for-{}", &effect.id()[7..15]), "current": { "prompt": payload[2], "revision": 1 } })
}

fn install(models: Calls, presents: Calls, forms: Vec<&'static str>) -> Install {
    Box::new(move |session: &mut Session<Record>| {
        let mut forms = forms.into_iter();
        session.provide("call/model", move |effect: &Effect| {
            models.lock().unwrap().push(effect.clone());
            Reply::Value(json!({ "form": forms.next().expect("unexpected model call") }))
        });
        session.provide("learning/present", move |effect: &Effect| {
            presents.lock().unwrap().push(effect.clone());
            Reply::Value(publication(effect))
        });
    })
}

fn host(pool: &PgPool, url: &str) -> OperatorHost {
    OperatorHost::new(pool.clone(), url, "capsule", Duration::from_secs(30))
}

fn path_of(ws: Uuid) -> Value {
    json!(["workspaces", ws.to_string(), "activities"])
}

#[sqlx::test(migrations = "../../migrations")]
async fn present_wait_follow_up_finish(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = host(&pool, &url(&pool).await);
    let (models, presents) = (Calls::default(), Calls::default());
    let handle = host
        .open(
            ws,
            &capsule::environment(ws),
            install(
                models.clone(),
                presents.clone(),
                vec![FIRST, FOLLOW_UP, FINISH],
            ),
        )
        .await
        .unwrap();
    let instance = handle
        .instantiate(capsule::compile(ws).unwrap())
        .unwrap()
        .unwrap();

    // The goal in: the model presents, the publication lands, the run parks
    // on the learner's wait with the publication beside the path.
    let started = handle
        .run_once("goal-1", instance, vec![json!(GOAL)])
        .unwrap()
        .unwrap();
    assert!(
        matches!(started.outcome(), Outcome::Parked(_)),
        "{started:?}"
    );
    let park = handle.pending().unwrap().remove(0);
    assert_eq!(park.family(), "learner/wait");
    assert_eq!(park.effect().payload()[0], path_of(ws));
    assert_eq!(
        park.effect().payload()[1]["current"]["prompt"],
        "What does the return G_t sum?"
    );
    {
        let presents = presents.lock().unwrap();
        assert_eq!(presents.len(), 1);
        assert_eq!(
            presents[0].payload(),
            &[
                path_of(ws),
                json!("recall"),
                json!("What does the return G_t sum?"),
                json!("the discounted sum of future rewards")
            ]
        );
    }
    assert_eq!(
        models.lock().unwrap()[0].payload()[0][3],
        GOAL,
        "the goal is the task"
    );

    // The learner answers (20b observes the attempt for the park): the
    // model reads it and proposes a follow-up, which parks again.
    let attempt = json!({ "attemptId": "attempt-1", "response": "the discounted sum of rewards from t onward" });
    let resumed = handle
        .resolve(park, Answer::Observe(Reply::Value(attempt.clone())))
        .unwrap()
        .unwrap();
    assert!(
        matches!(resumed.outcome(), Outcome::Parked(_)),
        "{resumed:?}"
    );
    let park = handle.pending().unwrap().remove(0);
    assert_eq!(park.family(), "learner/wait");
    assert_eq!(
        park.effect().payload()[1]["current"]["prompt"],
        "Why discount at all?"
    );
    assert_eq!(presents.lock().unwrap().len(), 2);
    {
        let models = models.lock().unwrap();
        assert_eq!(models.len(), 2);
        let turns = &models[1].payload()[0][4];
        assert_eq!(turns.as_array().unwrap().len(), 1, "one turn on the record");
        assert_eq!(
            turns[0][1][0],
            json!(["attempt", attempt]),
            "the attempt is what the verb answered"
        );
    }

    // The second attempt: the model finishes.
    let finished = handle
        .resolve(
            park,
            Answer::Observe(Reply::Value(
                json!({ "attemptId": "attempt-2", "response": "sooner is worth more" }),
            )),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        finished.outcome(),
        &Outcome::Value(json!([
            "done",
            "You can define the return and say why it is discounted."
        ])),
        "{finished:?}"
    );
    assert_eq!(models.lock().unwrap().len(), 3);
    assert_eq!(presents.lock().unwrap().len(), 2);
    assert!(handle.pending().unwrap().is_empty());
    host.close(ws).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_capsule_filled_for_another_workspace_is_refused_at_admission(pool: PgPool) {
    let ws = workspace(&pool).await;
    let other = workspace(&pool).await;
    let host = host(&pool, &url(&pool).await);
    let (models, presents) = (Calls::default(), Calls::default());
    let handle = host
        .open(
            ws,
            &capsule::environment(ws),
            install(models.clone(), presents.clone(), vec![FIRST]),
        )
        .await
        .unwrap();
    // A misbound program: compiled for `other`, offered to `ws`'s session.
    // Admission refuses it: the needs name scopes the grants do not cover.
    let refused = handle
        .instantiate(capsule::compile(other).unwrap())
        .unwrap()
        .unwrap_err();
    let why = format!("{refused:?}");
    assert!(
        why.contains("NotCovered") && why.contains(&other.to_string()),
        "{why}"
    );
    assert_eq!(presents.lock().unwrap().len(), 0, "no provider was asked");
    assert_eq!(models.lock().unwrap().len(), 0);
    assert!(
        handle.instances().unwrap().is_empty(),
        "nothing was instantiated"
    );
    host.close(ws).await.unwrap();
}
