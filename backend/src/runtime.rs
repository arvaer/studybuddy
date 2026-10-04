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
//! exists. Reopen itself asks no provider. The receipt also carries what
//! the learner read since the coach's last turn (21c): the page stays the
//! reading page reported, each delivered once.
//!
//! **A kill mid-think is resumed (20d).** A run interrupted at `call/model`
//! or `learning/present`, by the process dying or by the adapter answering
//! `unknown`, reopens as a park on that family with no receipt. Neither
//! effect is unsafe to serve again under the same id: the model has no side
//! effect outside the record, and `learning/present` commits its receipt
//! with the publication, so no receipt means nothing landed, and a reply
//! lost after the commit is answered from the receipt by the endpoint. So
//! the first state read after reopen allows each such park again
//! (`Interrupt(Allow)`, H3) and the think goes on where it stopped. Each
//! park is allowed once per process: a second failure of the same park
//! leaves it `stalled` until the next restart, so a broken model is not
//! called on every page poll.
//!
//! Providers: the model as this process has it (`operator::Model`, or a
//! test's scripted one) and `learning/present` and `learning/assess` (20f)
//! as clients of this process's own effect endpoints over loopback
//! (`operator::providers`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::services::attempt::AttemptService;
use capsule_corp::sdk::{Answer, Completed, Interrupt, Outcome, Park, Reply, Session};
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
const ASSESS_FAMILY: &str = "learning/assess";
const SOURCE_FAMILY: &str = "source/read";
/// How long a dead process keeps a workspace from its successor. Renewed
/// every `LEASE_RENEWAL` by a live one; short, because every backend
/// restart during a session is exactly this wait (20e).
pub const LEASE_TTL: Duration = Duration::from_secs(10);
pub const LEASE_RENEWAL: Duration = Duration::from_secs(3);
const WAIT_FAMILY: &str = "learner/wait";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "operator")]
pub enum OperatorState {
    /// No run in flight, nothing pending. `last` says how the last run
    /// ended, if one did; `summary` is the coach's closing summary when it
    /// finished (`coach/finish`), for the page to show whole.
    Idle {
        last: Option<String>,
        summary: Option<String>,
    },
    /// A run is in flight on the owner thread.
    Thinking,
    /// The run parked on the learner's attempt; the activity to answer.
    #[serde(rename_all = "camelCase")]
    Waiting { current_activity_id: Option<String> },
    /// Parked on something the learner cannot answer (the model, with no
    /// provider or uncertain): the host has to settle it.
    Stalled { families: Vec<String> },
    /// Another process holds the workspace's session lease, usually one
    /// that died within the last `LEASE_TTL`: the page keeps polling and the
    /// operator comes back when the lease lapses (20e).
    Unavailable { why: String },
}

pub struct OperatorRuntime {
    host: OperatorHost,
    pool: PgPool,
    base_url: String,
    secret: Arc<str>,
    installer: Installer,
    thinking: Mutex<HashSet<Uuid>>,
    /// Effect ids allowed again in this process after an interruption.
    allowed: Mutex<HashSet<String>>,
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
            host: OperatorHost::new(pool.clone(), database_url, "capsule", LEASE_TTL),
            pool,
            base_url: base_url.into(),
            secret,
            installer,
            thinking: Mutex::new(HashSet::new()),
            allowed: Mutex::new(HashSet::new()),
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
            for family in [PRESENT_FAMILY, ASSESS_FAMILY, SOURCE_FAMILY] {
                session.provide(
                    family,
                    operator::providers::effect_client(
                        &base_url,
                        workspace,
                        family,
                        Arc::clone(&secret),
                    ),
                );
            }
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
    /// background, once: the revision id is the run's dedup key. The run's
    /// one argument is the task text: the goal and, under it, the learner's
    /// sources (21a), what the coach may read, each by id, title and page
    /// count. Text, because the adapter's task is text.
    pub fn start(self: &Arc<Self>, workspace: Uuid, goal_revision: Uuid, intent: String) {
        self.think(workspace, "start", move |runtime| async move {
            let task = runtime.task(workspace, &intent).await?;
            let handle = runtime.open(workspace).await?;
            let run = tokio::task::spawn_blocking(move || -> Result<_, OperatorError> {
                // The instance of the capsule as it is in this build: the
                // recorded one when its definition is the same, else a new
                // one, so a change to the program reaches every workspace
                // at its next goal (and the record keeps the old instance).
                let compiled = capsule::compile(workspace)?;
                let definition = compiled.definition().address().to_string();
                let current = handle
                    .instances()?
                    .into_iter()
                    .find(|recorded| recorded.definition() == definition);
                let instance = match current {
                    Some(recorded) => recorded,
                    None => {
                        tracing::info!(workspace = %workspace, %definition, "instantiating the capsule as this build has it");
                        handle.instantiate(compiled)??
                    }
                };
                Ok(handle.run_once(&goal_revision.to_string(), instance, vec![json!(task)])??)
            })
            .await
            .map_err(|_| OperatorError::Join)??;
            Ok(describe(run.outcome()))
        });
    }

    /// The task text: the intent, then the learner's sources as the coach
    /// is told of them, one line each with id, title and page count, newest
    /// first, at most twenty. The intent alone when there are none.
    async fn task(&self, workspace: Uuid, intent: &str) -> Result<String, OperatorError> {
        let rows: Vec<(Uuid, String, Option<i32>)> = sqlx::query_as(
            "SELECT r.id, r.title, jsonb_array_length(r.content_pages)::int
             FROM resources r JOIN workspaces w ON w.user_id = r.user_id
             WHERE w.id = $1 AND r.content_pages IS NOT NULL
             ORDER BY r.added_at DESC LIMIT 20",
        )
        .bind(workspace)
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(intent.to_string());
        }
        let mut task =
            format!("{intent}\n\nSources the learner uploaded (read them with coach/read by id):");
        for (id, title, pages) in rows {
            task.push_str(&format!("\n- {id}: {title}, {} pages", pages.unwrap_or(0)));
        }
        Ok(task)
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
            let mut payload = serde_json::to_value(&receipt)
                .map_err(|e| repository(domain::errors::DomainError::Repository(e.to_string())))?;
            let mut tx = self.pool.begin().await?;
            // What the learner read since the coach's last turn (21c),
            // delivered with this receipt and marked so in the same
            // transaction, so each stay reaches the coach once.
            let reading: Vec<(Uuid, String, i32, i32)> = sqlx::query_as(
                "UPDATE page_views v SET delivered_at = now()
                 FROM resources r
                 WHERE v.resource_id = r.id AND v.workspace_id = $1 AND v.delivered_at IS NULL
                 RETURNING v.resource_id, r.title, v.page, v.seconds",
            )
            .bind(workspace)
            .fetch_all(&mut *tx)
            .await?;
            if !reading.is_empty() {
                let mut reading: Vec<_> = reading;
                reading.sort_by_key(|(_, _, page, _)| *page);
                payload["reading"] = Value::Array(
                    reading
                        .into_iter()
                        .map(|(id, title, page, seconds)| {
                            json!({ "resourceId": id, "title": title, "page": page, "seconds": seconds })
                        })
                        .collect(),
                );
            }
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

    /// The workspace's parks interrupted mid-think: pending on a family
    /// other than the learner's, with no receipt, and not yet allowed again
    /// by this process. Marks nothing.
    async fn interrupted(
        &self,
        workspace: Uuid,
        handle: &Handle<Record>,
    ) -> Result<Vec<Park>, OperatorError> {
        let mut interrupted = Vec::new();
        for park in handle.pending()? {
            let allowed = self
                .allowed
                .lock()
                .expect("allowed")
                .contains(park.digest());
            if park.family() == WAIT_FAMILY
                || allowed
                || receipts::find(&self.pool, workspace, park.digest())
                    .await?
                    .is_some()
            {
                continue;
            }
            interrupted.push(park);
        }
        Ok(interrupted)
    }

    /// Settle what the workspace is owed, in the background: every pending
    /// park with a receipt is completed from it (18c), every park
    /// interrupted mid-think is allowed again (20d), and the run goes on.
    /// Answers whether the workspace is now thinking.
    pub async fn poke(self: &Arc<Self>, workspace: Uuid) -> Result<bool, OperatorError> {
        if self.thinking.lock().expect("thinking").contains(&workspace) {
            return Ok(true);
        }
        let handle = self.open(workspace).await?;
        if self.owed(workspace, &handle).await?.is_empty()
            && self.interrupted(workspace, &handle).await?.is_empty()
        {
            return Ok(false);
        }
        Ok(self.think(workspace, "settle", move |runtime| async move {
            let mut settled = Vec::new();
            // A completion or an allow continues the run on the owner
            // thread, for as long as a think takes, so it goes on a blocking
            // task; and it may park on a new wait that already has an
            // attempt, so go round until nothing is owed.
            for _ in 0..8 {
                let handle = runtime.open(workspace).await?;
                let owed = runtime.owed(workspace, &handle).await?;
                let interrupted = runtime.interrupted(workspace, &handle).await?;
                if owed.is_empty() && interrupted.is_empty() {
                    break;
                }
                for park in interrupted {
                    let (family, digest) = (park.family().to_string(), park.digest().to_string());
                    runtime
                        .allowed
                        .lock()
                        .expect("allowed")
                        .insert(digest.clone());
                    tracing::info!(workspace = %workspace, effect = %digest, family = %family, uncertain = park.uncertain().unwrap_or("killed"), "interrupted think allowed again");
                    let handle = runtime.open(workspace).await?;
                    let run = tokio::task::spawn_blocking(move || {
                        handle.resolve(park, Answer::Interrupt(Interrupt::Allow))
                    })
                    .await
                    .map_err(|_| OperatorError::Join)???;
                    settled.push(format!("resumed {family}: {}", describe(run.outcome())));
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
        let thinking = match self.poke(workspace).await {
            Ok(thinking) => thinking,
            Err(OperatorError::Leased(_)) => {
                return Ok(OperatorState::Unavailable {
                    why: "another process held this workspace's session a moment ago; retrying"
                        .into(),
                })
            }
            Err(error) => return Err(error),
        };
        if thinking {
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
        let runs = handle.runs()?;
        let last = runs.last().map(|run| describe(run.outcome()));
        let summary = runs.last().and_then(|run| summary_of(run.outcome()));
        Ok(OperatorState::Idle { last, summary })
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

/// The coach's closing summary, when the run ended with `coach/finish`:
/// the value `("done" summary)`.
fn summary_of(outcome: &Outcome) -> Option<String> {
    let Outcome::Value(value) = outcome else {
        return None;
    };
    match value.as_array()?.as_slice() {
        [tag, summary] if tag == "done" => summary.as_str().map(str::to_string),
        _ => None,
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
