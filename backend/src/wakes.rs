//! Each wake of the operator, measured (21d): the context handed to the
//! model and what it cost. The capsule record already holds every
//! `call/model` request and reply whole; this is the measure beside it, one
//! row per wake in `wakes`, so the context policy (deferred until records
//! exist) is chosen from records.
//!
//! `Wakes::observe` wraps any `call/model` provider: it reads the size of
//! the request, how many turns it carries and how many of those are reads
//! of the source, times the call, and takes the API's `usage` and the verb
//! picked from the reply. The row goes down a channel to one writer task,
//! so the owner thread never waits on the database. `lugia wakes <id>`
//! prints a workspace's rows.

use std::time::Instant;

use capsule_corp::sdk::{Effect, Reply};
use serde_json::Value;
use sqlx::PgPool;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

/// One wake, as measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wake {
    pub workspace: Uuid,
    pub effect_id: String,
    pub latency_ms: i32,
    pub request_bytes: i32,
    pub turns: i32,
    pub reads: i32,
    pub input_tokens: Option<i32>,
    pub cache_read_tokens: Option<i32>,
    pub cache_write_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    /// The verb the coach picked (`coach/present`…), or how the call ended
    /// (`declined`, `refused`, `unknown`, `no-form`).
    pub picked: String,
}

/// The handle an installer gets for one workspace's session.
#[derive(Clone)]
pub struct Wakes {
    workspace: Uuid,
    sink: UnboundedSender<Wake>,
}

impl Wakes {
    /// The sender every session's handle clones, and the receiver the
    /// writer drains.
    pub fn channel() -> (UnboundedSender<Wake>, UnboundedReceiver<Wake>) {
        unbounded_channel()
    }

    pub fn new(workspace: Uuid, sink: UnboundedSender<Wake>) -> Self {
        Self { workspace, sink }
    }

    /// `provider`, measured: the same replies, one `Wake` per call.
    pub fn observe<P>(&self, mut provider: P) -> impl FnMut(&Effect) -> Reply
    where
        P: FnMut(&Effect) -> Reply,
    {
        let (workspace, sink) = (self.workspace, self.sink.clone());
        move |effect: &Effect| {
            let started = Instant::now();
            let reply = provider(effect);
            let wake = measure(
                workspace,
                effect.id(),
                effect.payload(),
                &reply,
                started.elapsed().as_millis() as i32,
            );
            let _ = sink.send(wake);
            reply
        }
    }
}

/// The measure of one call, from the effect's id and payload and its reply.
pub fn measure(
    workspace: Uuid,
    effect_id: &str,
    payload: &[Value],
    reply: &Reply,
    latency_ms: i32,
) -> Wake {
    let request_bytes = serde_json::to_vec(payload).map(|b| b.len()).unwrap_or(0) as i32;
    // ("agent.v2" system past task turns offered): turns is a list of
    // (reply results); a read's result is ("pages" …).
    let turns = payload
        .first()
        .and_then(|r| r.get(4))
        .and_then(Value::as_array);
    let reads = turns
        .map(|turns| turns.iter().filter(|turn| turn[1][0][0] == "pages").count())
        .unwrap_or(0) as i32;
    let turns = turns.map(Vec::len).unwrap_or(0) as i32;
    let (usage, picked) = match reply {
        Reply::Value(value) => {
            let picked = value["form"]
                .as_str()
                .map(|form| {
                    form.trim_start_matches('(')
                        .split_whitespace()
                        .next()
                        .unwrap_or("no-form")
                        .to_string()
                })
                .unwrap_or_else(|| "no-form".to_string());
            (value["usage"].clone(), picked)
        }
        Reply::Declined(_) => (Value::Null, "declined".into()),
        Reply::Refused(_) => (Value::Null, "refused".into()),
        Reply::Unknown(_) => (Value::Null, "unknown".into()),
    };
    let count = |key: &str| usage[key].as_i64().map(|n| n as i32);
    Wake {
        workspace,
        effect_id: effect_id.to_string(),
        latency_ms,
        request_bytes,
        turns,
        reads,
        input_tokens: count("input_tokens"),
        cache_read_tokens: count("cache_read_input_tokens"),
        cache_write_tokens: count("cache_creation_input_tokens"),
        output_tokens: count("output_tokens"),
        picked,
    }
}

/// Drain the channel into `wakes` until every sender is gone. One task per
/// process; a failed insert is logged and the next row still goes in.
pub async fn write(pool: PgPool, mut rows: UnboundedReceiver<Wake>) {
    while let Some(wake) = rows.recv().await {
        let written = sqlx::query(
            "INSERT INTO wakes (workspace_id, effect_id, latency_ms, request_bytes, turns, reads,
                                input_tokens, cache_read_tokens, cache_write_tokens, output_tokens, picked)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(wake.workspace)
        .bind(&wake.effect_id)
        .bind(wake.latency_ms)
        .bind(wake.request_bytes)
        .bind(wake.turns)
        .bind(wake.reads)
        .bind(wake.input_tokens)
        .bind(wake.cache_read_tokens)
        .bind(wake.cache_write_tokens)
        .bind(wake.output_tokens)
        .bind(&wake.picked)
        .execute(&pool)
        .await;
        match written {
            Ok(_) => {
                tracing::debug!(workspace = %wake.workspace, picked = %wake.picked, request_bytes = wake.request_bytes, turns = wake.turns, reads = wake.reads, input_tokens = ?wake.input_tokens, latency_ms = wake.latency_ms, "wake recorded")
            }
            Err(error) => {
                tracing::warn!(workspace = %wake.workspace, %error, "a wake was not recorded")
            }
        }
    }
}

/// `lugia wakes <workspace-id>`: the workspace's wakes, oldest first, one
/// line each, and the totals.
pub async fn print(pool: &PgPool, workspace: Uuid) -> Result<(), sqlx::Error> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        String,
        i32,
        i32,
        i32,
        i32,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        String,
    )> = sqlx::query_as(
        "SELECT to_char(woke_at, 'YYYY-MM-DD HH24:MI:SS'), latency_ms, request_bytes, turns, reads, input_tokens, cache_read_tokens,
                cache_write_tokens, output_tokens, picked
         FROM wakes WHERE workspace_id = $1 ORDER BY woke_at",
    )
    .bind(workspace)
    .fetch_all(pool)
    .await?;
    println!(
        "{:<20} {:>7} {:>9} {:>5} {:>5} {:>7} {:>7} {:>7} {:>6}  picked",
        "woke at", "ms", "bytes", "turns", "reads", "input", "cached", "written", "out"
    );
    let opt = |n: Option<i32>| n.map(|n| n.to_string()).unwrap_or_else(|| "-".into());
    let (mut bytes, mut input, mut cached, mut written, mut out_total) =
        (0i64, 0i64, 0i64, 0i64, 0i64);
    for (at, ms, b, turns, reads, inp, cr, cw, out, picked) in &rows {
        bytes += *b as i64;
        input += inp.unwrap_or(0) as i64;
        cached += cr.unwrap_or(0) as i64;
        written += cw.unwrap_or(0) as i64;
        out_total += out.unwrap_or(0) as i64;
        println!(
            "{:<20} {:>7} {:>9} {:>5} {:>5} {:>7} {:>7} {:>7} {:>6}  {}",
            at,
            ms,
            b,
            turns,
            reads,
            opt(*inp),
            opt(*cr),
            opt(*cw),
            opt(*out),
            picked
        );
    }
    println!(
        "{} wakes, {} request bytes; tokens: {} uncached input, {} read from cache, {} written to cache, {} output",
        rows.len(),
        bytes,
        input,
        cached,
        written,
        out_total
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_call_is_measured_from_its_request_and_reply() {
        let payload = vec![json!([
            "agent.v2",
            "system",
            [],
            "task",
            [
                ["r1", [["pages", { "found": true }]]],
                ["r2", [["attempt", { "attemptId": "a" }]]]
            ],
            []
        ])];
        let reply = Reply::Value(json!({
            "form": "(coach/present \"q\" \"a\")",
            "usage": { "input_tokens": 1200, "cache_read_input_tokens": 1000, "cache_creation_input_tokens": 50, "output_tokens": 80 }
        }));
        let wake = measure(Uuid::nil(), "sha256:e1", &payload, &reply, 4321);
        assert_eq!(
            (wake.turns, wake.reads, wake.picked.as_str()),
            (2, 1, "coach/present")
        );
        assert_eq!(
            (
                wake.input_tokens,
                wake.cache_read_tokens,
                wake.cache_write_tokens,
                wake.output_tokens
            ),
            (Some(1200), Some(1000), Some(50), Some(80))
        );
        assert!(wake.request_bytes > 50);
        assert_eq!(wake.latency_ms, 4321);
        let declined = measure(
            Uuid::nil(),
            "sha256:e2",
            &payload,
            &Reply::Declined("over".into()),
            1,
        );
        assert_eq!(
            (declined.picked.as_str(), declined.input_tokens),
            ("declined", None)
        );
    }
}
