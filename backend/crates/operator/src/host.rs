//! One capsule session per workspace, owned by one process at a time (18b).
//!
//! `OperatorHost` spawns a `capsule_host::owner::Owner` per workspace on
//! demand. The owner thread opens `PgStorage::existing` on the `capsule`
//! schema that migration 20240107 created, with a pool of its own (PgStorage
//! blocks on a private Tokio runtime and must never run on a worker), and
//! `Session::open` either starts the record or reopens it: the SDK walks the
//! ref if one exists and refuses an environment other than the recorded one.
//!
//! The lease is StudyBuddy's: `workspace_sessions` holds `owner_lease` and
//! `lease_until`. A process claims the row before it spawns the owner, renews
//! it while the owner lives, and clears it on `close`. A second process that
//! finds a live lease held by another owner gets `OperatorError::Leased` and
//! spawns nothing. A process that died leaves its lease to expire.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use capsule_corp::sdk::{
    Completed, Environment, EnvironmentError, Park, Reply, RunError, Session, SessionError,
    StorageError,
};
use capsule_host::owner::{Gone, Handle, Owner};
use capsule_host::postgres::PgStorage;
use sqlx::PgPool;
use uuid::Uuid;

use crate::receipts;

/// The storage every StudyBuddy session lives in.
pub type Record = PgStorage;

#[derive(Debug, thiserror::Error)]
pub enum OperatorError {
    #[error("workspace {0} is leased to another owner")]
    Leased(Uuid),
    #[error("workspace {0} has no session open in this process")]
    NotOpen(Uuid),
    #[error("environment: {0}")]
    Environment(#[from] EnvironmentError),
    #[error("record: {0}")]
    Storage(#[from] StorageError),
    #[error("session: {0}")]
    Session(#[from] SessionError),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("the session owner is gone: {0}")]
    Gone(#[from] Gone),
    #[error("the owner thread could not be joined")]
    Join,
    #[error("run: {0}")]
    Run(#[from] RunError),
    #[error("capsule: {0}")]
    Compile(#[from] capsule_corp::sdk::CompileError),
    #[error("instantiate: {0}")]
    Instantiate(#[from] capsule_corp::sdk::InstantiateError),
}

/// A pending effect settled from its receipt on reconcile.
#[derive(Clone, Debug, PartialEq)]
pub struct Settled {
    pub effect_id: String,
    pub family: String,
    pub completed: Completed,
}

/// What reconciling a workspace's pending effects came to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reconciled {
    /// Completed from a receipt: performed before, reply now on the record
    /// (or already there).
    pub settled: Vec<Settled>,
    /// Pending with no receipt: never performed, or performed and not
    /// committed. The caller dispatches them, or denies them, or, for one
    /// the provider left `uncertain`, allows it again under the same id.
    pub unsettled: Vec<Park>,
}

/// Installs providers (and a router, if any) on a freshly opened session,
/// on the owner thread. Nothing it captures has to be `Sync`.
pub type Install = Box<dyn FnOnce(&mut Session<Record>) + Send>;

/// Holds this process's sessions and their leases.
pub struct OperatorHost {
    pool: PgPool,
    database_url: String,
    schema: String,
    owner_id: Uuid,
    lease_ttl: Duration,
    owners: Mutex<HashMap<Uuid, Arc<Owner<Record>>>>,
}

impl OperatorHost {
    /// `pool` is the application pool, used for the lease rows only. The
    /// record is opened over `database_url` in `schema` by each owner thread.
    pub fn new(
        pool: PgPool,
        database_url: impl Into<String>,
        schema: impl Into<String>,
        lease_ttl: Duration,
    ) -> Self {
        Self {
            pool,
            database_url: database_url.into(),
            schema: schema.into(),
            owner_id: Uuid::new_v4(),
            lease_ttl,
            owners: Mutex::new(HashMap::new()),
        }
    }

    /// This process's identity in `workspace_sessions.owner_lease`.
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    /// The session name a workspace's record is kept under.
    pub fn session_name(workspace: Uuid) -> String {
        format!("workspace/{workspace}")
    }

    /// Open (or reopen) the workspace's session: claim the lease, then
    /// spawn its owner, which opens the record and runs `install`. Opening a
    /// workspace this process already holds answers the existing handle.
    pub async fn open(
        &self,
        workspace: Uuid,
        environment: &str,
        install: Install,
    ) -> Result<Handle<Record>, OperatorError> {
        if let Some(owner) = self.owners.lock().expect("owners").get(&workspace) {
            return Ok(owner.handle());
        }
        let session_name = self.claim(workspace).await?;
        let environment = Environment::compile(environment)?;
        let (url, schema) = (self.database_url.clone(), self.schema.clone());
        let spawned = tokio::task::spawn_blocking(move || {
            let name = session_name.clone();
            Owner::spawn(
                &name,
                64,
                move || -> Result<Session<Record>, OperatorError> {
                    let storage = PgStorage::existing(&url, &schema)?;
                    let mut session = Session::open(storage, &session_name, environment)?;
                    install(&mut session);
                    Ok(session)
                },
            )
        })
        .await
        .map_err(|_| OperatorError::Join)?;
        let owner = match spawned {
            Ok(owner) => Arc::new(owner),
            Err(error) => {
                self.release(workspace).await?;
                return Err(error);
            }
        };
        let handle = owner.handle();
        self.owners.lock().expect("owners").insert(workspace, owner);
        Ok(handle)
    }

    /// The handle of a session this process holds.
    pub fn handle(&self, workspace: Uuid) -> Result<Handle<Record>, OperatorError> {
        self.owners
            .lock()
            .expect("owners")
            .get(&workspace)
            .map(|owner| owner.handle())
            .ok_or(OperatorError::NotOpen(workspace))
    }

    /// Stop the owner after the commands it has accepted, then clear the
    /// lease. The record stays; `open` reopens it.
    pub async fn close(&self, workspace: Uuid) -> Result<(), OperatorError> {
        let owner = self
            .owners
            .lock()
            .expect("owners")
            .remove(&workspace)
            .ok_or(OperatorError::NotOpen(workspace))?;
        let owner = Arc::try_unwrap(owner).map_err(|_| OperatorError::Join)?;
        tokio::task::spawn_blocking(move || owner.stop())
            .await
            .map_err(|_| OperatorError::Join)?;
        self.release(workspace).await
    }

    /// Settle what the record owes an answer from what the application
    /// performed (H3, the four crash points): every pending park whose
    /// effect id has a receipt in this workspace is completed with the
    /// receipt's payload; the record answers `Recorded` if the reply lands
    /// now and `Already` if it landed before the crash. Parks without a
    /// receipt are handed back untouched. Reopen never asks a provider, so
    /// call this after `open` and before dispatching anything.
    pub async fn reconcile(&self, workspace: Uuid) -> Result<Reconciled, OperatorError> {
        let handle = self.handle(workspace)?;
        let mut reconciled = Reconciled::default();
        for park in handle.pending()? {
            match receipts::find(&self.pool, workspace, park.digest()).await? {
                Some(receipt) => {
                    let completed =
                        handle.complete(park.digest(), Reply::Value(receipt.payload))??;
                    reconciled.settled.push(Settled {
                        effect_id: receipt.effect_id,
                        family: receipt.family,
                        completed,
                    });
                }
                None => reconciled.unsettled.push(park),
            }
        }
        Ok(reconciled)
    }

    /// Push every held lease out by the TTL. Call it on an interval shorter
    /// than the TTL; a lease that lapses lets another process claim the
    /// workspace while this one still holds the owner (the tell in the plan).
    pub async fn renew_leases(&self) -> Result<u64, OperatorError> {
        let held: Vec<Uuid> = self
            .owners
            .lock()
            .expect("owners")
            .keys()
            .copied()
            .collect();
        if held.is_empty() {
            return Ok(0);
        }
        let renewed = sqlx::query(
            "UPDATE workspace_sessions
             SET lease_until = now() + make_interval(secs => $3)
             WHERE owner_lease = $1 AND workspace_id = ANY($2)",
        )
        .bind(self.owner_id)
        .bind(&held)
        .bind(self.lease_ttl.as_secs_f64())
        .execute(&self.pool)
        .await?;
        Ok(renewed.rows_affected())
    }

    /// Take the lease, or extend one this owner already holds, or take over
    /// one that lapsed. Answers the session name on the row.
    async fn claim(&self, workspace: Uuid) -> Result<String, OperatorError> {
        let row: Option<(String,)> = sqlx::query_as(
            "INSERT INTO workspace_sessions (workspace_id, session_name, owner_lease, lease_until)
             VALUES ($1, $2, $3, now() + make_interval(secs => $4))
             ON CONFLICT (workspace_id) DO UPDATE
                 SET owner_lease = EXCLUDED.owner_lease,
                     lease_until = EXCLUDED.lease_until
                 WHERE workspace_sessions.owner_lease IS NULL
                    OR workspace_sessions.owner_lease = EXCLUDED.owner_lease
                    OR workspace_sessions.lease_until <= now()
             RETURNING session_name",
        )
        .bind(workspace)
        .bind(Self::session_name(workspace))
        .bind(self.owner_id)
        .bind(self.lease_ttl.as_secs_f64())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|(name,)| name)
            .ok_or(OperatorError::Leased(workspace))
    }

    async fn release(&self, workspace: Uuid) -> Result<(), OperatorError> {
        sqlx::query(
            "UPDATE workspace_sessions SET owner_lease = NULL, lease_until = NULL
             WHERE workspace_id = $1 AND owner_lease = $2",
        )
        .bind(workspace)
        .bind(self.owner_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
