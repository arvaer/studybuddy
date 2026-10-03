//! The operator's model: upstream's Claude adapter, in-process on the owner
//! thread (19b; docs/phase-2-build.md, 19b).
//!
//! `capsule_host::claude::Claude` is the reference `call/model` provider:
//! it maps the verbs `offer` lists onto the Messages API's tools, keeps
//! thinking and signatures across turns, marks cache breakpoints, declines
//! a conversation the API refuses, and answers the one form `act` reads.
//! StudyBuddy writes no adapter; it installs that one on each session, the
//! way upstream's launcher does with its own `claude` connector. The model
//! has no side effect outside the record, so it needs no receipt: a park on
//! `call/model` after a crash is allowed again under the same id (H3).
//!
//! The key is `ANTHROPIC_API_KEY`, the variable capsule-corp reads. It
//! lives here and in the transport's closure, and nowhere a log can reach.

use std::fmt;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

use capsule_corp::sdk::{Reply, Session};
use capsule_host::claude::{Claude, Transport};
use serde_json::{json, Value as Json};

use crate::host::Record;

/// The family the capsule applies.
pub const FAMILY: &str = "call/model";

/// The model when `OPERATOR_MODEL` is unset: the launcher's default too.
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

/// Where the adapter posts. Fixed: no request can change it.
const MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";

/// The adapter's stream opener: the API's request id, kept on the message
/// as `request_id` so a refusal can name it (upstream `claude.rs`).
const REQUEST: &str = "capsule.request";

/// A model the owner installs on every session it opens. Clone and `Send`:
/// the adapter itself is built on the owner thread, where it lives.
#[derive(Clone)]
pub struct Model {
    key: Arc<str>,
    model: String,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("key", &"<redacted>")
            .field("model", &self.model)
            .finish()
    }
}

impl Model {
    pub fn new(key: impl Into<Arc<str>>, model: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            model: model.into(),
        }
    }

    /// The model name, for a log line. Never the key.
    pub fn name(&self) -> &str {
        &self.model
    }

    /// The adapter over HTTPS. Call it on the owner thread: the adapter is
    /// not `Send`.
    pub fn provider(&self) -> Claude {
        Claude::with_transport(&self.model, https(Arc::clone(&self.key)))
    }

    /// Install `call/model` on a freshly opened session; the body of an
    /// `Install` (host.rs).
    pub fn install(&self, session: &mut Session<Record>) {
        session.provide(FAMILY, self.provider());
    }
}

/// The Messages API over HTTPS: upstream's own transport (`Claude::new`),
/// without the stderr echo of thinking and text, which a server must not
/// write where its logs go. A failed send is `unknown` (the request may
/// have reached the API); an error status is a refusal with what the API
/// said and no body of ours.
fn https(key: Arc<str>) -> Transport {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(600)))
        .http_status_as_error(false)
        .build()
        .into();
    Box::new(move |body: &Json| {
        let response = agent
            .post(MESSAGES_URL)
            .header("x-api-key", &*key)
            .header("anthropic-version", "2023-06-01")
            .send_json(body)
            .map_err(|error| Reply::Unknown(format!("claude: {error}")))?;
        let status = response.status();
        let id = response
            .headers()
            .get("request-id")
            .and_then(|id| id.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let mut body = response.into_body();
        if !status.is_success() {
            let error: Json = body.read_json().unwrap_or(Json::Null);
            let message = error["error"]["message"].as_str().unwrap_or("no message");
            return Err(Reply::Refused(format!("claude: HTTP {status}: {message}")));
        }
        let opened = format!("data: {}\n", json!({ "type": REQUEST, "id": id }));
        let stream = std::io::Read::chain(std::io::Cursor::new(opened), body.into_reader());
        Ok(Box::new(BufReader::new(stream)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_names_the_model_and_never_the_key() {
        let model = Model::new("sk-ant-very-secret", DEFAULT_MODEL);
        let text = format!("{model:?}");
        assert!(
            text.contains(DEFAULT_MODEL) && text.contains("<redacted>"),
            "{text}"
        );
        assert!(!text.contains("very-secret"));
        assert_eq!(model.name(), "claude-opus-5-5");
    }
}
