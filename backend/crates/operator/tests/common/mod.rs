//! Shared by the operator's integration tests: the probe's capsule and
//! environment, scripted providers, a workspace row, the test database URL.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use capsule_corp::sdk::{Effect, Reply, Session};
use operator::host::{Install, Record};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub const CAPSULE: &str =
    include_str!("../../../../../experiments/capsule-operator/learning.capsule");
pub const ENVIRONMENT: &str = r#"
(environment studybuddy-18b
  (grant capability call/model :kind model-call)
  (grant capability learning/present :kind tool :scope "workspaces/rl/*")
  (grant capability learner/answer :kind tool :scope "workspaces/rl/*")
  (require constraint max-bytes :kind structural :rule (rule (max-bytes 16384)))
  (budget :evaluator-steps 20000 :boundary-effects 12))
"#;
pub const ASK: &str = r#"(coach/ask "workspaces/rl/lesson" "A gives 2 and ends; B gives 0 then 5. Which has greater return?")"#;

pub type Calls = Arc<Mutex<Vec<Effect>>>;

/// The test database's URL: `DATABASE_URL` with the database `#[sqlx::test]` made.
pub async fn url(pool: &PgPool) -> String {
    let (name,): (String,) = sqlx::query_as("SELECT current_database()")
        .fetch_one(pool)
        .await
        .unwrap();
    let base = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let (prefix, _) = base.rsplit_once('/').expect("a database in DATABASE_URL");
    format!("{prefix}/{name}")
}

pub async fn workspace(pool: &PgPool) -> Uuid {
    let (user,): (Uuid,) = sqlx::query_as(
        "INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'A') RETURNING id",
    )
    .bind(format!("{}@example.com", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();
    let (ws,): (Uuid,) =
        sqlx::query_as("INSERT INTO workspaces (user_id) VALUES ($1) RETURNING id")
            .bind(user)
            .fetch_one(pool)
            .await
            .unwrap();
    ws
}

/// What a `learning/present` provider observes for `effect`.
pub fn publication(effect: &Effect) -> serde_json::Value {
    json!(["publication.v1", effect.id(), effect.payload()[0], 1])
}

/// A scripted model answering `forms` in order, and a `learning/present`
/// provider answering a publication receipt.
pub fn install(models: Calls, presents: Calls, forms: Vec<&'static str>) -> Install {
    Box::new(move |session: &mut Session<Record>| {
        model(session, models, forms);
        session.provide("learning/present", move |effect: &Effect| {
            presents.lock().unwrap().push(effect.clone());
            Reply::Value(publication(effect))
        });
    })
}

/// The scripted model alone: `learning/present` has no provider at the door,
/// so the run parks on it and the host performs it elsewhere (H3, "Where
/// the host runs").
pub fn model_only(models: Calls, forms: Vec<&'static str>) -> Install {
    Box::new(move |session: &mut Session<Record>| model(session, models, forms))
}

/// The scripted model, and a `learning/present` provider that cannot say
/// whether it performed the effect (`Reply::Unknown`) the first time it is
/// asked and answers a receipt after.
pub fn present_unknown_once(models: Calls, presents: Calls, forms: Vec<&'static str>) -> Install {
    Box::new(move |session: &mut Session<Record>| {
        model(session, models, forms);
        session.provide("learning/present", move |effect: &Effect| {
            let mut seen = presents.lock().unwrap();
            seen.push(effect.clone());
            if seen.len() == 1 {
                Reply::Unknown("publication committed; reply delivery unknown".into())
            } else {
                Reply::Value(publication(effect))
            }
        });
    })
}

fn model(session: &mut Session<Record>, models: Calls, forms: Vec<&'static str>) {
    let mut forms = forms.into_iter();
    session.provide("call/model", move |effect: &Effect| {
        models.lock().unwrap().push(effect.clone());
        Reply::Value(json!({"form": forms.next().expect("unexpected model call")}))
    });
}

pub async fn lease(pool: &PgPool, ws: Uuid) -> (Option<Uuid>, bool) {
    let (owner, live): (Option<Uuid>, Option<bool>) = sqlx::query_as(
        "SELECT owner_lease, lease_until > now() FROM workspace_sessions WHERE workspace_id = $1",
    )
    .bind(ws)
    .fetch_one(pool)
    .await
    .unwrap();
    (owner, live.unwrap_or(false))
}
