//! 18c: receipts settle the record through the four crash points of H3
//! (sdk "Providers", "The park authorizes dispatch"), on the real database.
//! The host performs `learning/present` outside the session, as 19a will:
//! no provider at the door, the run parks, the application writes a receipt
//! by effect id, and `reconcile` completes the park from it after a restart.
mod common;

use std::time::Duration;

use capsule_corp::sdk::{Answer, Capsule, Completed, Interrupt, Outcome, Reply, RunError};
use capsule_host::owner::Handle;
use common::{
    model_only, present_unknown_once, publication, url, workspace, Calls, ASK, CAPSULE, ENVIRONMENT,
};
use operator::host::Record;
use operator::receipts::{self, Receipt, Recorded};
use operator::OperatorHost;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn new_host(pool: &PgPool, url: &str) -> OperatorHost {
    OperatorHost::new(pool.clone(), url, "capsule", Duration::from_secs(30))
}

/// Start the probe's lesson; it parks on `learning/present` when no
/// provider answers at the door.
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

fn receipt(ws: Uuid, park: &capsule_corp::sdk::Park) -> Receipt {
    Receipt {
        effect_id: park.digest().to_string(),
        workspace_id: ws,
        family: park.family().to_string(),
        payload: publication(park.effect()),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn before_dispatch_nothing_happened_and_the_host_may_deny(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![ASK]))
        .await
        .unwrap();
    let started = start(&handle);
    assert!(matches!(started.outcome(), Outcome::Parked(_)));
    let park = handle.pending().unwrap().remove(0);
    assert_eq!(park.family(), "learning/present");
    assert_eq!(park.uncertain(), None);

    // Crash before the host performed it; reopen; nothing to settle.
    host.close(ws).await.unwrap();
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![]))
        .await
        .unwrap();
    let reconciled = host.reconcile(ws).await.unwrap();
    assert!(reconciled.settled.is_empty());
    assert_eq!(reconciled.unsettled, vec![park.clone()]);

    // The host will not perform it: the run ends refused and asks no one.
    let run = handle
        .resolve(park, Answer::Interrupt(Interrupt::Deny))
        .unwrap()
        .unwrap();
    assert!(
        matches!(run.outcome(), Outcome::Refused(_)),
        "{:?}",
        run.outcome()
    );
    assert!(handle.pending().unwrap().is_empty());
    let (receipts,): (i64,) = sqlx::query_as("SELECT count(*) FROM effect_receipts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(receipts, 0);
    host.close(ws).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn during_possible_delivery_the_receipt_settles_it_or_the_host_allows_the_same_id(
    pool: PgPool,
) {
    let ws = workspace(&pool).await;
    let (models, presents) = (Calls::default(), Calls::default());
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(
            ws,
            ENVIRONMENT,
            present_unknown_once(models.clone(), presents.clone(), vec![ASK]),
        )
        .await
        .unwrap();
    start(&handle);
    let park = handle.pending().unwrap().remove(0);
    assert_eq!(park.family(), "learning/present");
    assert!(park.uncertain().is_some(), "the provider did not know");
    assert_eq!(presents.lock().unwrap().len(), 1);

    // Case A: the application had in fact committed; its receipt exists.
    // The crash happens here; reopen settles from the receipt.
    assert_eq!(
        receipts::record(&mut pool.acquire().await.unwrap(), &receipt(ws, &park))
            .await
            .unwrap(),
        Recorded::New
    );
    host.close(ws).await.unwrap();
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(
            ws,
            ENVIRONMENT,
            present_unknown_once(models.clone(), presents.clone(), vec![]),
        )
        .await
        .unwrap();
    let reconciled = host.reconcile(ws).await.unwrap();
    assert_eq!(reconciled.settled.len(), 1);
    assert_eq!(reconciled.settled[0].effect_id, park.digest());
    assert!(matches!(
        reconciled.settled[0].completed,
        Completed::Recorded(_)
    ));
    assert!(reconciled.unsettled.is_empty());
    let next = handle.pending().unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(
        next[0].family(),
        "learner/answer",
        "the lesson went on to wait for the learner"
    );
    assert_eq!(
        presents.lock().unwrap().len(),
        1,
        "nothing was re-performed"
    );
    host.close(ws).await.unwrap();

    // Case B, a second workspace: unknown delivery and no receipt. The host
    // allows it again under the same effect id; the provider is asked once
    // more with that id and nothing else.
    let ws2 = workspace(&pool).await;
    let (models2, presents2) = (Calls::default(), Calls::default());
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(
            ws2,
            ENVIRONMENT,
            present_unknown_once(models2, presents2.clone(), vec![ASK]),
        )
        .await
        .unwrap();
    start(&handle);
    let park = handle.pending().unwrap().remove(0);
    assert!(park.uncertain().is_some());
    assert!(
        host.reconcile(ws2).await.unwrap().settled.is_empty(),
        "no receipt, nothing to settle"
    );
    // It may already have happened, so a bare allow is refused (capsule-corp
    // F-011); a retry performs it again.
    assert!(
        handle
            .resolve(park.clone(), Answer::Interrupt(Interrupt::Allow))
            .unwrap()
            .is_err(),
        "an uncertain park refuses a bare allow"
    );
    handle
        .resolve(park.clone(), Answer::Interrupt(Interrupt::Retry))
        .unwrap()
        .unwrap();
    {
        let asked = presents2.lock().unwrap();
        assert_eq!(asked.len(), 2);
        assert_eq!(
            asked[1].id(),
            park.digest(),
            "same id: the handler's idempotency key"
        );
    }
    assert_eq!(handle.pending().unwrap()[0].family(), "learner/answer");
    host.close(ws2).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn after_the_domain_commit_the_receipt_completes_the_park_on_reopen(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![ASK]))
        .await
        .unwrap();
    start(&handle);
    let park = handle.pending().unwrap().remove(0);

    // The host performs the effect: domain change and receipt commit
    // together. Then it dies before completing the park.
    let mut tx = pool.begin().await.unwrap();
    assert_eq!(
        receipts::record(&mut tx, &receipt(ws, &park))
            .await
            .unwrap(),
        Recorded::New
    );
    tx.commit().await.unwrap();
    host.close(ws).await.unwrap();

    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![]))
        .await
        .unwrap();
    assert_eq!(
        handle.pending().unwrap(),
        vec![park.clone()],
        "reopened pending again"
    );
    let reconciled = host.reconcile(ws).await.unwrap();
    assert!(matches!(
        reconciled.settled[0].completed,
        Completed::Recorded(_)
    ));
    assert_eq!(handle.pending().unwrap()[0].family(), "learner/answer");

    // A receipt from another workspace settles nothing here.
    let other = workspace(&pool).await;
    assert!(receipts::find(&pool, other, park.digest())
        .await
        .unwrap()
        .is_none());
    host.close(ws).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn after_the_reply_is_recorded_the_same_completion_is_already(pool: PgPool) {
    let ws = workspace(&pool).await;
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![ASK]))
        .await
        .unwrap();
    start(&handle);
    let park = handle.pending().unwrap().remove(0);
    let receipt = receipt(ws, &park);
    receipts::record(&mut pool.acquire().await.unwrap(), &receipt)
        .await
        .unwrap();
    let first = host.reconcile(ws).await.unwrap();
    let Completed::Recorded(run) = &first.settled[0].completed else {
        panic!("first completion lands")
    };

    // The receipt is written again (a retried provider): same row answers.
    assert_eq!(
        receipts::record(
            &mut pool.acquire().await.unwrap(),
            &Receipt {
                payload: json!("other"),
                ..receipt.clone()
            }
        )
        .await
        .unwrap(),
        Recorded::Existing(receipt.clone())
    );

    // Completing again, live and after a restart, writes nothing.
    let again = handle
        .complete(park.digest(), Reply::Value(receipt.payload.clone()))
        .unwrap()
        .unwrap();
    assert_eq!(again, Completed::Already(run.clone()));
    let before = handle.inspect().unwrap();
    host.close(ws).await.unwrap();
    let host = new_host(&pool, &url(&pool).await);
    let handle = host
        .open(ws, ENVIRONMENT, model_only(Calls::default(), vec![]))
        .await
        .unwrap();
    assert_eq!(handle.inspect().unwrap(), before);
    assert!(
        host.reconcile(ws).await.unwrap().settled.is_empty(),
        "nothing pending has a receipt"
    );
    assert_eq!(
        handle
            .complete(park.digest(), Reply::Value(receipt.payload.clone()))
            .unwrap()
            .unwrap(),
        Completed::Already(run.clone())
    );
    assert!(
        matches!(
            handle
                .complete(park.digest(), Reply::Value(json!("other")))
                .unwrap(),
            Err(RunError::Completed(_))
        ),
        "a conflicting observation is refused"
    );
    assert_eq!(handle.inspect().unwrap(), before, "retries append nothing");

    host.close(ws).await.unwrap();
}
