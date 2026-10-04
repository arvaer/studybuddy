//! 18b: a session survives a restart on the real database, and one owner at
//! a time holds a workspace. The probe's capsule and scripted providers,
//! PgStorage in the `capsule` schema that migration 20240107 made.

mod common;

use std::time::Duration;

use capsule_corp::sdk::{Capsule, Outcome};
use capsule_host::owner::Handle;
use common::{install, lease, url, workspace, Calls, ASK, CAPSULE, ENVIRONMENT};
use operator::host::Record;
use operator::{OperatorError, OperatorHost};
use serde_json::json;
use sqlx::PgPool;

/// Start under the dedup id: the first time on a fresh instance, after a
/// reopen on the instance the record already holds (a second instantiate
/// would be a new instance and so a different run).
fn start(handle: &Handle<Record>) -> capsule_corp::sdk::Run {
    let instance = match handle.instances().unwrap().into_iter().next() {
        Some(recorded) => recorded,
        None => handle
            .instantiate(Capsule::compile(CAPSULE).unwrap())
            .unwrap()
            .unwrap(),
    };
    handle
        .run_once(
            "start-1",
            instance,
            vec![json!("Practice reward versus return.")],
        )
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_session_reopens_from_postgres_after_its_owner_stops(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = OperatorHost::new(
        pool.clone(),
        url(&pool).await,
        "capsule",
        Duration::from_secs(30),
    );
    let (models, presents) = (Calls::default(), Calls::default());

    let handle = host
        .open(
            ws,
            ENVIRONMENT,
            install(models.clone(), presents.clone(), vec![ASK]),
        )
        .await
        .unwrap();
    let started = start(&handle);
    assert!(matches!(started.outcome(), Outcome::Parked(_)));
    let parked = handle.pending().unwrap();
    assert_eq!(parked.len(), 1);
    assert_eq!(parked[0].family(), "learner/answer");
    let before = handle.inspect().unwrap();
    assert_eq!(lease(&pool, ws).await, (Some(host.owner_id()), true));

    host.close(ws).await.unwrap();
    assert!(handle.pending().is_err(), "the old handle is gone");
    assert_eq!(lease(&pool, ws).await, (None, false));
    let (nodes,): (i64,) = sqlx::query_as("SELECT count(*) FROM capsule.nodes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(nodes > 0, "the record is in the capsule schema");

    // A new process: a new host, a new owner id, the same database.
    let again = OperatorHost::new(
        pool.clone(),
        url(&pool).await,
        "capsule",
        Duration::from_secs(30),
    );
    let handle = again
        .open(
            ws,
            ENVIRONMENT,
            install(models.clone(), presents.clone(), vec![]),
        )
        .await
        .unwrap();
    assert_eq!(handle.inspect().unwrap(), before, "same record");
    assert_eq!(handle.pending().unwrap(), parked, "same park");
    assert_eq!(handle.runs().unwrap().len(), 1);
    assert_eq!(
        start(&handle).form(),
        started.form(),
        "same start answers the recorded run"
    );
    assert_eq!(models.lock().unwrap().len(), 1, "reopen asked no provider");
    assert_eq!(presents.lock().unwrap().len(), 1);
    again.close(ws).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn one_owner_at_a_time_until_the_lease_lapses(pool: PgPool) {
    let ws = workspace(&pool).await;
    let first = OperatorHost::new(
        pool.clone(),
        url(&pool).await,
        "capsule",
        Duration::from_secs(30),
    );
    let second = OperatorHost::new(
        pool.clone(),
        url(&pool).await,
        "capsule",
        Duration::from_secs(30),
    );
    let quiet = || install(Calls::default(), Calls::default(), vec![]);

    first.open(ws, ENVIRONMENT, quiet()).await.unwrap();
    assert!(matches!(
        second.open(ws, ENVIRONMENT, quiet()).await,
        Err(OperatorError::Leased(w)) if w == ws
    ));
    assert!(matches!(second.handle(ws), Err(OperatorError::NotOpen(_))));

    assert_eq!(first.renew_leases().await.unwrap(), 1);

    // The first process dies without releasing; its lease runs out.
    sqlx::query("UPDATE workspace_sessions SET lease_until = now() - interval '1 second' WHERE workspace_id = $1")
        .bind(ws)
        .execute(&pool)
        .await
        .unwrap();
    second.open(ws, ENVIRONMENT, quiet()).await.unwrap();
    assert_eq!(lease(&pool, ws).await, (Some(second.owner_id()), true));
    assert_eq!(
        first.renew_leases().await.unwrap(),
        0,
        "a lost lease is not renewed"
    );

    second.close(ws).await.unwrap();
    first.close(ws).await.unwrap();
}

/// The environment gains a grant (as 20f's `learning/assess` did): the
/// record under the old one cannot be reopened under the new, so the host
/// opens a fresh record for the workspace, idle, and the old record stays.
#[sqlx::test(migrations = "../../migrations")]
async fn a_changed_environment_opens_a_fresh_record_and_keeps_the_old(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = OperatorHost::new(
        pool.clone(),
        url(&pool).await,
        "capsule",
        Duration::from_secs(30),
    );
    let (models, presents) = (Calls::default(), Calls::default());
    let handle = host
        .open(
            ws,
            ENVIRONMENT,
            install(models.clone(), presents.clone(), vec![ASK]),
        )
        .await
        .unwrap();
    start(&handle);
    assert_eq!(handle.pending().unwrap().len(), 1);
    host.close(ws).await.unwrap();

    // One more grant: a different environment address.
    let grown = ENVIRONMENT.replace(
        "(grant capability call/model",
        "(grant capability learning/assess :kind tool :scope \"workspaces/rl/*\")\n  (grant capability call/model",
    );
    assert_ne!(grown, ENVIRONMENT);
    let handle = host
        .open(
            ws,
            &grown,
            install(models.clone(), presents.clone(), vec![ASK]),
        )
        .await
        .expect("a changed environment opens, on a fresh record");
    assert!(
        handle.pending().unwrap().is_empty(),
        "the old park is not carried over"
    );
    assert!(handle.runs().unwrap().is_empty());
    assert_eq!(
        models.lock().unwrap().len(),
        1,
        "no provider asked by the reopen"
    );

    // Both records are in storage (refs are keyed by a hash of the name):
    // the old one is still there, and the old environment still opens it.
    let refs: i64 = sqlx::query_scalar("SELECT count(*) FROM capsule.refs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(refs, 2, "two session refs");
    host.close(ws).await.unwrap();
    let old = host
        .open(
            ws,
            ENVIRONMENT,
            install(models.clone(), presents.clone(), vec![ASK]),
        )
        .await
        .unwrap();
    assert_eq!(
        old.pending().unwrap().len(),
        1,
        "the old record is untouched"
    );
    host.close(ws).await.unwrap();
}
