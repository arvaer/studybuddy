//! 19b: the operator thinks with upstream's Claude adapter in-process on the
//! owner thread. A capsule offers one verb, the kernel adds its parameters
//! from the lambda, the adapter turns it into the API's tool, a fake
//! transport answers one streamed tool call, and the run finishes with the
//! form the call named. Real SDK, real PgStorage, no network.

mod common;

use std::io::{BufRead, Cursor};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use capsule_corp::sdk::{Capsule, Outcome, Reply, Session};
use capsule_host::claude::{Claude, Transport, KIND};
use common::{url, workspace};
use operator::host::{Install, Record};
use operator::model::FAMILY;
use operator::OperatorHost;
use serde_json::{json, Value as Json};
use sqlx::PgPool;

/// One turn: the model is offered `coach/finish`, whose one parameter the
/// kernel reads off the lambda, and is expected to call it.
const CAPSULE: &str = r#"
(capsule model-19b
  (purpose "one model turn through the adapter")
  (use capability call/model :kind model-call)
  (define coach/system "Finish the task in one call.")
  (define coach/verbs (quote ((coach/finish "Finish with a short summary."))))
  (define coach/finish (lambda (summary) (list "done" summary)))
  (define coach/entry
    (lambda (task)
      (car (act (call/model (list "agent.v2" coach/system (list) task (list) (offer coach/verbs)))
                coach/verbs))))
  (entry coach/entry))
"#;

const ENVIRONMENT: &str = r#"
(environment studybuddy-19b
  (grant capability call/model :kind model-call)
  (require constraint max-bytes :kind structural :rule (rule (max-bytes 16384)))
  (budget :evaluator-steps 20000 :boundary-effects 4))
"#;

type Bodies = Arc<Mutex<Vec<Json>>>;

fn sse(events: &[Json]) -> String {
    events
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            )
        })
        .collect()
}

/// The API's stream for one `coach__finish` call, as the adapter reads it.
fn finish_stream() -> String {
    sse(&[
        json!({ "type": "message_start", "message": { "role": "assistant", "content": [], "usage": { "input_tokens": 40 } } }),
        json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "thinking", "thinking": "", "signature": "" } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "thinking_delta", "thinking": "Done." } }),
        json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "signature_delta", "signature": "EqQB" } }),
        json!({ "type": "content_block_stop", "index": 0 }),
        json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "tool_use", "id": "toolu_1", "name": "coach__finish", "input": {} } }),
        json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "{\"summary\": \"the learner " } }),
        json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "has it\"}" } }),
        json!({ "type": "content_block_stop", "index": 1 }),
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" }, "usage": { "output_tokens": 9 } }),
        json!({ "type": "message_stop" }),
    ])
}

/// The adapter over a transport that keeps each request body and answers
/// the finish stream. Built on the owner thread, like `Model::install`.
fn install(bodies: Bodies) -> Install {
    Box::new(move |session: &mut Session<Record>| {
        let transport: Transport = Box::new(move |body: &Json| {
            bodies.lock().unwrap().push(body.clone());
            Ok(Box::new(Cursor::new(finish_stream())) as Box<dyn BufRead>)
        });
        session.provide(FAMILY, Claude::with_transport("test-model", transport));
    })
}

/// A transport the API refuses: the adapter's `Refused` ends the run.
fn install_refusing() -> Install {
    Box::new(|session: &mut Session<Record>| {
        let transport: Transport =
            Box::new(|_: &Json| Err(Reply::Refused("claude: HTTP 401: invalid x-api-key".into())));
        session.provide(FAMILY, Claude::with_transport("test-model", transport));
    })
}

fn host(pool: &PgPool, url: &str) -> OperatorHost {
    OperatorHost::new(pool.clone(), url, "capsule", Duration::from_secs(30))
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_adapter_turns_the_offered_verb_into_a_tool_and_its_call_into_the_form(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = host(&pool, &url(&pool).await);
    let bodies = Bodies::default();
    let handle = host
        .open(ws, ENVIRONMENT, install(bodies.clone()))
        .await
        .unwrap();

    let instance = handle
        .instantiate(Capsule::compile(CAPSULE).unwrap())
        .unwrap()
        .unwrap();
    let run = handle
        .run_once("start-1", instance, vec![json!("Say the learner has it.")])
        .unwrap()
        .unwrap();
    assert_eq!(
        run.outcome(),
        &Outcome::Value(json!(["done", "the learner has it"])),
        "{run:?}"
    );

    // What the API was sent: the verb as a tool with its parameter, the
    // system prompt, the task; the model name from the adapter.
    let body = {
        let bodies = bodies.lock().unwrap();
        assert_eq!(bodies.len(), 1);
        bodies[0].clone()
    };
    assert_eq!(body["model"], "test-model");
    assert_eq!(body["system"][0]["text"], "Finish the task in one call.");
    assert_eq!(body["tools"][0]["name"], "coach__finish");
    assert_eq!(
        body["tools"][0]["input_schema"]["required"],
        json!(["summary"])
    );
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(
        body["messages"][0]["content"][0]["text"],
        "Say the learner has it."
    );
    assert!(
        !body.to_string().contains(KIND),
        "the request kind is the adapter's, not the API's"
    );

    // What the record holds: the reply with its one form, beside the
    // adapter's transcript, which only the adapter reads again next turn.
    let (replies,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM capsule.nodes
         WHERE node::text LIKE '%(coach/finish %the learner has it%' AND node::text LIKE '%toolu_1%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        replies, 1,
        "the form and the transcript are on the record together"
    );
    host.close(ws).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_refusal_from_the_api_ends_the_run_as_refused(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, install_refusing())
        .await
        .unwrap();
    let instance = handle
        .instantiate(Capsule::compile(CAPSULE).unwrap())
        .unwrap()
        .unwrap();
    let run = handle
        .run_once("start-1", instance, vec![json!("Say anything.")])
        .unwrap()
        .unwrap();
    match run.outcome() {
        Outcome::Refused(why) => assert!(why.contains("HTTP 401"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    host.close(ws).await.unwrap();
}
