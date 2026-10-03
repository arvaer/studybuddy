//! The owner's providers for the effects the application performs (20a):
//! one-line clients of the effect endpoints over loopback, in upstream's
//! connector protocol (sdk-surface "The connector protocol", L1c), so the
//! day the launcher's `http` connector replaces the embedded owner, the
//! endpoints need no change.
//!
//! The request is the effect as JSON with the id as `Idempotency-Key`; the
//! reply is one of `value`, `refused`, `declined`, `unknown` from a 2xx
//! body. Anything else (no connection, another status, an unreadable body)
//! is `unknown`: the effect may have committed, so the run parks uncertain
//! and the receipt table settles it on reconcile (H3).

use std::sync::Arc;
use std::time::Duration;

use capsule_corp::sdk::{Effect, Reply};
use serde_json::{json, Value as Json};

/// A provider for `family` that posts each effect to
/// `{base_url}/internal/effects/{workspace}/{path}`, where `path` is the
/// family with its slash as a dot. Blocking, as a provider on the owner
/// thread is.
pub fn effect_client(
    base_url: &str,
    workspace: uuid::Uuid,
    family: &str,
    secret: Arc<str>,
) -> impl FnMut(&Effect) -> Reply {
    let url = format!(
        "{}/internal/effects/{workspace}/{}",
        base_url.trim_end_matches('/'),
        family.replace('/', ".")
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    move |effect: &Effect| {
        let body = json!({
            "id": effect.id(),
            "capability": effect.capability(),
            "payload": effect.payload(),
        });
        let response = match agent
            .post(&url)
            .header("authorization", &format!("Bearer {secret}"))
            .header("idempotency-key", effect.id())
            .send_json(&body)
        {
            Ok(response) => response,
            Err(error) => return Reply::Unknown(format!("effect endpoint: {error}")),
        };
        let status = response.status();
        if !status.is_success() {
            return Reply::Unknown(format!("effect endpoint answered {status}"));
        }
        let reply: Json = match response.into_body().read_json() {
            Ok(reply) => reply,
            Err(error) => {
                return Reply::Unknown(format!("effect endpoint: unreadable reply: {error}"))
            }
        };
        read(reply)
    }
}

/// The protocol's one object as a `Reply`.
fn read(mut reply: Json) -> Reply {
    let why = |v: &Json| v.as_str().unwrap_or("no reason given").to_string();
    if let Some(value) = reply.get_mut("value") {
        Reply::Value(value.take())
    } else if let Some(r) = reply.get("refused") {
        Reply::Refused(why(r))
    } else if let Some(d) = reply.get("declined") {
        Reply::Declined(why(d))
    } else if let Some(u) = reply.get("unknown") {
        Reply::Unknown(why(u))
    } else {
        Reply::Unknown(format!("effect endpoint: reply has no form: {reply}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_reply_forms_and_nothing_else() {
        assert_eq!(
            read(json!({ "value": { "id": 1 } })),
            Reply::Value(json!({ "id": 1 }))
        );
        assert_eq!(
            read(json!({ "refused": "no" })),
            Reply::Refused("no".into())
        );
        assert_eq!(
            read(json!({ "declined": "later" })),
            Reply::Declined("later".into())
        );
        assert_eq!(
            read(json!({ "unknown": "maybe" })),
            Reply::Unknown("maybe".into())
        );
        assert!(matches!(read(json!({ "ok": true })), Reply::Unknown(_)));
    }
}
