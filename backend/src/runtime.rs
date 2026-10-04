//! The operator at run time (20a, 20b): this process's sessions, opened on
//! demand, each running the learning capsule for its workspace's goal.
//!
//! A goal starts a run in the background and the workspace is `thinking`
//! until the run parks or ends; a park on `learner/wait` is `waiting`, with
//! the activity the learner is to answer; no run in flight and nothing
//! pending is `idle`, with how the last run ended. The owner thread is busy
//! for the whole of a think, so `state` never asks it while one is in
//! flight.
//!
//! **An answer wakes the operator (20b).** The attempt the learner records
//! is what a `learner/wait` park waits for, so the attempt is its receipt:
//! when one exists against the revision the park holds, a receipt under
//! the park's effect id is written and the park completed with it, in the
//! background, and the run goes on to the next `present`. The same check
//! runs on every state read, so an attempt recorded while the process was
//! down, or in the gap before its receipt, is picked up on the next page
//! read, and reconcile (18c) completes any park whose receipt already
//! exists. Reopen itself asks no provider.
//!
//! Providers: the model as this process has it (`operator::Model`, or a
//! test's scripted one) and `learning/present` as a client of this
//! process's own effect endpoint over loopback (`operator::providers`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::services::attempt::AttemptService;
use capsule_corp::sdk::{Completed, Outcome, Park, Reply, Session};
use capsule_host::owner::Handle;
use infra::repositories::attempt::PgAttemptRepository;
use infra::repositories::workspace::PgWorkspaceRepository;
use operator::capsule;
use operator::host::{Install, Record};
use operator::receipts::{self, Receipt, Recorded};
use operator::{OperatorError, OperatorHost};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
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
    pool: PgPool,
    base_url: String,
    secret: Arc<str>,
    installer: Installer,
    thinking: Mutex<HashSet<Uuid>>,
}

impl OperatorRuntime {
    /// `base_url` is where this process's own `/internal/effects` answers,
    /// `secret` what it accepts.
    pub fn new(
        pool: PgPool,
        database_url: impl Into<String>,
        base_url: impl Into<String>,
        secret: Arc<str>,
        installer: Installer,
    ) -> Self {
        Self {
            host: OperatorHost::new(
                pool.clone(),
                database_url,
                "capsule",
                Duration::from_secs(60),
            ),
            pool,
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
    /// with this process's providers. Idempotent while open; asks no
    /// provider.
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
        self.host
            .open(workspace, &capsule::environment(workspace), install)
            .await
    }

    /// Mark the workspace thinking and run `work` in the background; the
    /// mark comes off when it ends. Answers whether it was started (false
    /// while a think is already in flight).
    fn think<F, Fut>(self: &Arc<Self>, workspace: Uuid, what: &'static str, work: F) -> bool
    where
        F: FnOnce(Arc<Self>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<String, OperatorError>> + Send,
    {
        if !self.thinking.lock().expect("thinking").insert(workspace) {
            return false;
        }
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            let outcome = work(Arc::clone(&runtime)).await;
            runtime
                .thinking
                .lock()
                .expect("thinking")
                .remove(&workspace);
            match outcome {
                Ok(outcome) => {
                    tracing::info!(workspace = %workspace, what, outcome = %outcome, "operator settled")
                }
                Err(error) => {
                    tracing::error!(workspace = %workspace, what, error = %error, "operator failed")
                }
            }
        });
        true
    }

    /// Start the run that serves `goal_revision` with `intent`, in the
    /// background, once: the revision id is the run's dedup key.
    pub fn start(self: &Arc<Self>, workspace: Uuid, goal_revision: Uuid, intent: String) {
        self.think(workspace, "start", move |runtime| async move {
            let handle = runtime.open(workspace).await?;
            let run =
                tokio::task::spawn_blocking(move || -> Result<_, OperatorError> {
                    let instance = match handle.instances()?.into_iter().next() {
                        Some(recorded) => recorded,
                        None => handle.instantiate(capsule::compile(workspace)?)??,
                    };
                    Ok(handle.run_once(
                        &goal_revision.to_string(),
                        instance,
                        vec![json!(intent)],
                    )??)
                })
                .await
                .map_err(|_| OperatorError::Join)??;
            Ok(describe(run.outcome()))
        });
    }

    /// What the workspace's pending parks are owed: a receipt already on
    /// the table, or, for a `learner/wait`, an attempt against its revision,
    /// which becomes its receipt now. Writes receipts, completes nothing.
    async fn owed(
        &self,
        workspace: Uuid,
        handle: &Handle<Record>,
    ) -> Result<Vec<Park>, OperatorError> {
        let pending = handle.pending()?;
        if pending.is_empty() {
            return Ok(Vec::new());
        }
        let workspaces = PgWorkspaceRepository::new(self.pool.clone());
        let mut owed = Vec::new();
        for park in pending {
            if receipts::find(&self.pool, workspace, park.digest())
                .await?
                .is_some()
            {
                owed.push(park);
                continue;
            }
            if park.family() != WAIT_FAMILY {
                continue;
            }
            let Some(revision) = revision_of(&park) else {
                continue;
            };
            let owner = workspaces.owner(workspace).await.map_err(repository)?;
            let Some(attempt) = workspaces
                .first_attempt(owner, revision)
                .await
                .map_err(repository)?
            else {
                continue;
            };
            let receipt = AttemptService::new(PgAttemptRepository::new(self.pool.clone()))
                .get(owner, attempt)
                .await
                .map_err(|e| repository(domain::errors::DomainError::Repository(e.to_string())))?;
            let payload = serde_json::to_value(&receipt)
                .map_err(|e| repository(domain::errors::DomainError::Repository(e.to_string())))?;
            let mut tx = self.pool.begin().await?;
            let recorded = receipts::record(
                &mut tx,
                &Receipt {
                    effect_id: park.digest().to_string(),
                    workspace_id: workspace,
                    family: WAIT_FAMILY.into(),
                    payload,
                },
            )
            .await?;
            tx.commit().await?;
            if let Recorded::New = recorded {
                tracing::info!(workspace = %workspace, effect = park.digest(), attempt = %attempt, "attempt receipted for the operator's wait");
            }
            owed.push(park);
        }
        Ok(owed)
    }

    /// Settle what the workspace is owed, in the background: every pending
    /// park with a receipt is completed from it (18c) and the run goes on.
    /// Answers whether the workspace is now thinking.
    pub async fn poke(self: &Arc<Self>, workspace: Uuid) -> Result<bool, OperatorError> {
        if self.thinking.lock().expect("thinking").contains(&workspace) {
            return Ok(true);
        }
        let handle = self.open(workspace).await?;
        if self.owed(workspace, &handle).await?.is_empty() {
            return Ok(false);
        }
        Ok(self.think(workspace, "settle", move |runtime| async move {
            let mut settled = Vec::new();
            // A completion continues the run on the owner thread, for as
            // long as a think takes, so it goes on a blocking task; and it
            // may park on a new wait that already has an attempt, so go
            // round until nothing is owed.
            for _ in 0..8 {
                let handle = runtime.open(workspace).await?;
                let owed = runtime.owed(workspace, &handle).await?;
                if owed.is_empty() {
                    break;
                }
                for park in owed {
                    let Some(receipt) =
                        receipts::find(&runtime.pool, workspace, park.digest()).await?
                    else {
                        continue;
                    };
                    let handle = runtime.open(workspace).await?;
                    let completed = tokio::task::spawn_blocking(move || {
                        handle.complete(park.digest(), Reply::Value(receipt.payload))
                    })
                    .await
                    .map_err(|_| OperatorError::Join)???;
                    let how = match &completed {
                        Completed::Recorded(run) => describe(run.outcome()),
                        Completed::Already(run) => format!("already: {}", describe(run.outcome())),
                    };
                    settled.push(how);
                }
            }
            Ok(settled.join("; "))
        }))
    }

    /// Where the workspace's operator is, for the learner's page. Settles
    /// first whatever is owed, so a page read is enough to wake it.
    pub async fn state(self: &Arc<Self>, workspace: Uuid) -> Result<OperatorState, OperatorError> {
        if self.poke(workspace).await? {
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

/// The revision a `learner/wait` park holds: the publication's current one.
fn revision_of(park: &Park) -> Option<Uuid> {
    let publication: &Value = park.effect().payload().get(1)?;
    publication["current"]["id"].as_str()?.parse().ok()
}

fn repository(e: domain::errors::DomainError) -> OperatorError {
    OperatorError::Database(sqlx::Error::Protocol(e.to_string()))
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
