//! The operator at run time (20a): this process's sessions, opened on
//! demand, each running the learning capsule for its workspace's goal.
//!
//! A goal starts a run in the background and the workspace is `thinking`
//! until the run parks or ends; a park on `learner/wait` is `waiting`, with
//! the activity the learner is to answer; no run in flight and nothing
//! pending is `idle`, with how the last run ended. The owner thread is busy
//! for the whole of a think, so `state` never asks it while one is in
//! flight. Opening a workspace reopens its record if one exists and
//! reconciles pending effects against the receipt table first (18c), so a
//! restart reads `waiting` from the record, asking no provider.
//!
//! Providers: the model as this process has it (`operator::Model`, or a
//! test's scripted one) and `learning/present` as a client of this
//! process's own effect endpoint over loopback (`operator::providers`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use capsule_corp::sdk::{Outcome, Session};
use capsule_host::owner::Handle;
use operator::capsule;
use operator::host::{Install, Record};
use operator::{OperatorError, OperatorHost};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

/// What installs the model on a fresh session: `operator::Model::install`
/// in the server, a scripted provider in tests.
pub type Installer = Arc<dyn Fn(&mut Session<Record>) + Send + Sync>;

const PRESENT_FAMILY: &str = "learning/present";
const WAIT_FAMILY: &str = "learner/wait";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "operator")]
pub enum OperatorState {
    /// No run in flight, nothing pending. `last` says how the last run
    /// ended, if one did.
    Idle { last: Option<String> },
    /// A run is in flight on the owner thread.
    Thinking,
    /// The run parked on the learner's attempt; the activity to answer.
    #[serde(rename_all = "camelCase")]
    Waiting { current_activity_id: Option<String> },
    /// Parked on something the learner cannot answer (the model, with no
    /// provider or uncertain): the host has to settle it.
    Stalled { families: Vec<String> },
}

pub struct OperatorRuntime {
    host: OperatorHost,
    base_url: String,
    secret: Arc<str>,
    installer: Installer,
    thinking: Mutex<HashSet<Uuid>>,
}

impl OperatorRuntime {
    /// `base_url` is where this process's own `/internal/effects` answers,
    /// `secret` what it accepts.
    pub fn new(
        pool: sqlx::PgPool,
        database_url: impl Into<String>,
        base_url: impl Into<String>,
        secret: Arc<str>,
        installer: Installer,
    ) -> Self {
        Self {
            host: OperatorHost::new(pool, database_url, "capsule", Duration::from_secs(60)),
            base_url: base_url.into(),
            secret,
            installer,
            thinking: Mutex::new(HashSet::new()),
        }
    }

    /// Push every held lease out; call it on an interval shorter than the
    /// TTL.
    pub async fn renew_leases(&self) -> Result<u64, OperatorError> {
        self.host.renew_leases().await
    }

    /// Stop the workspace's owner and release its lease; the record stays.
    pub async fn close(&self, workspace: Uuid) -> Result<(), OperatorError> {
        self.host.close(workspace).await
    }

    /// The workspace's session, opened or reopened under its environment
    /// with this process's providers, reconciled. Idempotent while open.
    async fn open(&self, workspace: Uuid) -> Result<Handle<Record>, OperatorError> {
        let installer = Arc::clone(&self.installer);
        let (base_url, secret) = (self.base_url.clone(), Arc::clone(&self.secret));
        let install: Install = Box::new(move |session: &mut Session<Record>| {
            installer(session);
            session.provide(
                PRESENT_FAMILY,
                operator::providers::effect_client(&base_url, workspace, PRESENT_FAMILY, secret),
            );
        });
        let handle = self
            .host
            .open(workspace, &capsule::environment(workspace), install)
            .await?;
        let reconciled = self.host.reconcile(workspace).await?;
        for settled in &reconciled.settled {
            tracing::info!(workspace = %workspace, effect = %settled.effect_id, family = %settled.family, "effect settled from its receipt");
        }
        Ok(handle)
    }

    /// Start the run that serves `goal_revision` with `intent`, in the
    /// background, once: the revision id is the run's dedup key.
    pub fn start(self: &Arc<Self>, workspace: Uuid, goal_revision: Uuid, intent: String) {
        if !self.thinking.lock().expect("thinking").insert(workspace) {
            return;
        }
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            let outcome = runtime.run(workspace, goal_revision, intent).await;
            runtime
                .thinking
                .lock()
                .expect("thinking")
                .remove(&workspace);
            match outcome {
                Ok(outcome) => {
                    tracing::info!(workspace = %workspace, outcome = %outcome, "operator run settled")
                }
                Err(error) => {
                    tracing::error!(workspace = %workspace, error = %error, "operator run failed")
                }
            }
        });
    }

    async fn run(
        &self,
        workspace: Uuid,
        goal_revision: Uuid,
        intent: String,
    ) -> Result<String, OperatorError> {
        let handle = self.open(workspace).await?;
        let run = tokio::task::spawn_blocking(move || -> Result<_, OperatorError> {
            let instance = match handle.instances()?.into_iter().next() {
                Some(recorded) => recorded,
                None => handle.instantiate(capsule::compile(workspace)?)??,
            };
            Ok(handle.run_once(&goal_revision.to_string(), instance, vec![json!(intent)])??)
        })
        .await
        .map_err(|_| OperatorError::Join)??;
        Ok(describe(run.outcome()))
    }

    /// Where the workspace's operator is, for the learner's page.
    pub async fn state(&self, workspace: Uuid) -> Result<OperatorState, OperatorError> {
        if self.thinking.lock().expect("thinking").contains(&workspace) {
            return Ok(OperatorState::Thinking);
        }
        let handle = self.open(workspace).await?;
        let pending = handle.pending()?;
        if let Some(wait) = pending.iter().find(|park| park.family() == WAIT_FAMILY) {
            let activity = wait
                .effect()
                .payload()
                .get(1)
                .and_then(|p| p["id"].as_str());
            return Ok(OperatorState::Waiting {
                current_activity_id: activity.map(str::to_string),
            });
        }
        if !pending.is_empty() {
            return Ok(OperatorState::Stalled {
                families: pending
                    .iter()
                    .map(|park| park.family().to_string())
                    .collect(),
            });
        }
        let last = handle.runs()?.last().map(|run| describe(run.outcome()));
        Ok(OperatorState::Idle { last })
    }
}

/// How a run ended, in a line.
fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Value(value) => format!("done: {value}"),
        Outcome::Refused(why) => format!("refused: {why}"),
        Outcome::Exhausted(what) => format!("exhausted: {what:?}"),
        Outcome::Parked(park) => format!("parked on {}", park.family()),
        Outcome::Paused(park) => format!("paused at {}", park.family()),
    }
}
