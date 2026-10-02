//! Real SDK, scripted model, in-memory publication receipts. No API or database.
use std::{cell::RefCell, rc::Rc};

use capsule_corp::sdk::{
    Answer, Capsule, Effect, Environment, Instance, MemoryStorage, Outcome, Park, Reply, RunError,
    Session,
};
use serde_json::{Value, json};

type Calls = Rc<RefCell<Vec<Effect>>>;

const ENVIRONMENT: &str = r#"
(environment studybuddy-probe
  (grant capability call/model :kind model-call)
  (grant capability learning/present :kind tool :scope "workspaces/rl/*")
  (grant capability learner/answer :kind tool :scope "workspaces/rl/*")
  (require constraint max-bytes :kind structural :rule (rule (max-bytes 16384)))
  (budget :evaluator-steps 20000 :boundary-effects 12))
"#;
const ASK: &str = r#"(coach/ask "workspaces/rl/lesson" "A gives 2 and ends; B gives 0 then 5. No discounting. Which has greater return, and why?")"#;
const FOLLOW_UP: &str = r#"(coach/propose "workspaces/rl/follow-up" "Compute both returns before choosing; include the delayed reward.")"#;
const FINISH: &str = r#"(coach/finish "Follow-up proposed; mastery remains unassessed.")"#;

fn open() -> (Session<MemoryStorage>, Instance) {
    let mut session = Session::open(
        MemoryStorage::new(),
        "learner-1/rl",
        Environment::compile(ENVIRONMENT).unwrap(),
    )
    .unwrap();
    let instance = session
        .instantiate(&Capsule::compile(include_str!("../learning.capsule")).unwrap())
        .unwrap();
    (session, instance)
}

fn model(session: &mut Session<MemoryStorage>, forms: &[&str], calls: &Calls) {
    let mut forms = forms
        .iter()
        .map(|form| form.to_string())
        .collect::<Vec<_>>()
        .into_iter();
    let calls = Rc::clone(calls);
    session.provide("call/model", move |effect: &Effect| {
        calls.borrow_mut().push(effect.clone());
        Reply::Value(json!({"form": forms.next().expect("unexpected model call")}))
    });
}

fn present(session: &mut Session<MemoryStorage>, calls: &Calls, uncertain: bool) {
    let calls = Rc::clone(calls);
    session.provide("learning/present", move |effect: &Effect| {
        assert_eq!(effect.payload().len(), 2);
        assert!(effect.payload()[1].is_string());
        calls.borrow_mut().push(effect.clone());
        if uncertain {
            Reply::Unknown("publication committed; reply delivery unknown".into())
        } else {
            Reply::Value(receipt(effect))
        }
    });
}

fn receipt(effect: &Effect) -> Value {
    json!(["publication.v1", effect.id(), effect.payload()[0], 1])
}

fn pending(session: &Session<MemoryStorage>, family: &str) -> Park {
    assert_eq!(session.pending().len(), 1);
    let park = session.pending()[0].clone();
    assert_eq!(park.family(), family);
    park
}

#[test]
fn lesson_resumes_after_reopen_and_duplicate_start_does_no_work() {
    let (mut session, instance) = open();
    let models = Calls::default();
    let publications = Calls::default();
    model(&mut session, &[ASK], &models);
    present(&mut session, &publications, false);
    let args = [json!("Practice reward versus return in workspaces/rl.")];
    let started = session.run_once("start-1", &instance, &args).unwrap();
    let park = pending(&session, "learner/answer");
    assert!(matches!(started.outcome(), Outcome::Parked(_)));
    assert_eq!(publications.borrow().len(), 1, "present before waiting");

    let before = session.inspect();
    let mut session = Session::reopen(session.into_storage(), "learner-1/rl").unwrap();
    assert_eq!(session.inspect(), before);
    assert_eq!(pending(&session, "learner/answer"), park);
    assert_eq!(models.borrow().len(), 1);
    model(&mut session, &[FOLLOW_UP, FINISH], &models);
    present(&mut session, &publications, false);
    let explanation = "I chose A because its immediate reward is higher.";
    let finished = session
        .resolve(&park, Answer::Reply(explanation.into()))
        .unwrap();
    assert_eq!(
        finished.outcome(),
        &Outcome::Value(json!([
            "done",
            "Follow-up proposed; mastery remains unassessed."
        ]))
    );
    assert!(
        json!(models.borrow()[1].payload())
            .to_string()
            .contains(explanation)
    );
    assert_eq!(
        publications.borrow()[1].payload()[0],
        "workspaces/rl/follow-up"
    );

    let runs = session.runs().to_vec();
    let before = session.inspect();
    let mut session = Session::reopen(session.into_storage(), "learner-1/rl").unwrap();
    assert_eq!(session.runs(), runs);
    assert_eq!(
        session
            .run_once("start-1", &instance, &args)
            .unwrap()
            .form(),
        started.form()
    );
    assert!(matches!(
        session.run_once("start-1", &instance, &[json!("different")]),
        Err(RunError::InputConflict(_))
    ));
    assert!(matches!(
        session.resolve(&park, Answer::Reply("again".into())),
        Err(RunError::NotPending(_))
    ));
    assert_eq!(
        session.inspect(),
        before,
        "replay and retries append nothing"
    );
    assert_eq!(models.borrow().len(), 3);
    assert_eq!(publications.borrow().len(), 2);
}

#[test]
fn a_model_cannot_present_in_another_workspace() {
    let (mut session, instance) = open();
    let publications = Calls::default();
    model(
        &mut session,
        &[r#"(coach/ask "workspaces/other/lesson" "Read this")"#],
        &Calls::default(),
    );
    present(&mut session, &publications, false);
    let run = session
        .run_once("outside", &instance, &[json!("practice")])
        .unwrap();
    assert!(matches!(run.outcome(), Outcome::Refused(_)));
    assert!(
        publications.borrow().is_empty(),
        "authority checked before provider"
    );
    assert!(session.pending().is_empty());
}

#[test]
fn uncertain_publication_resumes_from_receipt_without_republishing() {
    let (mut session, instance) = open();
    let models = Calls::default();
    let publications = Calls::default();
    model(&mut session, &[FOLLOW_UP], &models);
    present(&mut session, &publications, true);
    session
        .run_once("publish-1", &instance, &[json!("propose practice")])
        .unwrap();
    let park = pending(&session, "learning/present");
    assert!(park.uncertain().is_some());
    let existing_receipt = receipt(&publications.borrow()[0]);
    assert_eq!(existing_receipt[1], park.digest());

    let mut session = Session::reopen(session.into_storage(), "learner-1/rl").unwrap();
    assert_eq!(pending(&session, "learning/present"), park);
    model(&mut session, &[FINISH], &models);
    session.provide("learning/present", |_: &Effect| -> Reply {
        panic!("must reconcile, not republish")
    });
    let finished = session
        .resolve(&park, Answer::Observe(Reply::Value(existing_receipt)))
        .unwrap();
    assert!(matches!(finished.outcome(), Outcome::Value(_)));
    assert!(session.pending().is_empty());
    assert_eq!(publications.borrow().len(), 1);
    assert_eq!(models.borrow().len(), 2);
    let runs = session.runs().to_vec();
    let session = Session::reopen(session.into_storage(), "learner-1/rl").unwrap();
    assert_eq!(session.runs(), runs);
}
