//! Server-owned model access.
//!
//! The provider, model, credential, destination and timeout are fixed at
//! startup from `LLM_*` variables (see `config.rs`). Handlers pass messages
//! only; nothing from a request can change where the backend connects or
//! what it sends as a credential.

use std::fmt;
use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse, Json};
use reqwest::Url;
use serde::Serialize;
use serde_json::json;

use app::dtos::llm::{LlmMessage, LlmRole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmProvider {
    Anthropic,
    /// OpenAI's chat-completions shape; also used by Ollama and similar.
    OpenAi,
}

impl LlmProvider {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            _ => None,
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::OpenAi => "https://api.openai.com",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }
}

/// Everything the client needs, resolved and validated at startup.
#[derive(Clone)]
pub struct LlmSettings {
    pub provider: LlmProvider,
    pub model: String,
    /// Never log this.
    pub api_key: String,
    /// Scheme and host were checked against the allowlist in `config.rs`.
    pub base_url: Url,
    pub timeout: Duration,
}

impl fmt::Debug for LlmSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmSettings")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url.as_str())
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// Upper bound on `max_tokens` a request may ask for.
pub const MAX_TOKENS_CAP: u32 = 8192;
pub const MAX_TOKENS_DEFAULT: u32 = 2048;

#[derive(Debug)]
pub enum LlmError {
    /// No `LLM_*` configuration on this server.
    NotConfigured,
    /// The request body was rejected before anything left the server.
    Validation(String),
    /// The provider did not answer within the configured timeout.
    Timeout,
    /// The provider could not be reached.
    Transport,
    /// The provider answered with a non-success status.
    Provider(u16),
    /// The provider answered 2xx but not in the shape we expect.
    Malformed,
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "AI provider is not configured on this server"),
            Self::Validation(msg) => write!(f, "{msg}"),
            Self::Timeout => write!(f, "AI provider timed out"),
            Self::Transport => write!(f, "AI provider could not be reached"),
            Self::Provider(status) => write!(f, "AI provider returned an error (status {status})"),
            Self::Malformed => write!(f, "AI provider returned an unexpected response"),
        }
    }
}

impl std::error::Error for LlmError {}

impl IntoResponse for LlmError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Transport | Self::Provider(_) | Self::Malformed => StatusCode::BAD_GATEWAY,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

#[derive(Clone)]
pub struct LlmClient {
    settings: LlmSettings,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(settings: LlmSettings) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .timeout(settings.timeout)
            .build()?;
        Ok(Self { settings, http })
    }

    pub fn provider(&self) -> LlmProvider {
        self.settings.provider
    }

    pub fn model(&self) -> &str {
        &self.settings.model
    }

    /// Send one chat completion and return the assistant text.
    pub async fn complete(
        &self,
        messages: &[LlmMessage],
        max_tokens: Option<u32>,
    ) -> Result<String, LlmError> {
        if messages.is_empty() {
            return Err(LlmError::Validation("messages must not be empty".into()));
        }
        if messages.iter().any(|m| m.content.trim().is_empty()) {
            return Err(LlmError::Validation(
                "message content must not be empty".into(),
            ));
        }
        let max_tokens = max_tokens.unwrap_or(MAX_TOKENS_DEFAULT);
        if max_tokens == 0 || max_tokens > MAX_TOKENS_CAP {
            return Err(LlmError::Validation(format!(
                "maxTokens must be 1..={MAX_TOKENS_CAP}"
            )));
        }

        let s = &self.settings;
        let (path, body, header) = match s.provider {
            LlmProvider::Anthropic => {
                // Anthropic takes the system prompt outside the messages array.
                let system: Vec<&str> = messages
                    .iter()
                    .filter(|m| m.role == LlmRole::System)
                    .map(|m| m.content.as_str())
                    .collect();
                let chat: Vec<&LlmMessage> = messages
                    .iter()
                    .filter(|m| m.role != LlmRole::System)
                    .collect();
                let mut body =
                    json!({ "model": s.model, "messages": chat, "max_tokens": max_tokens });
                if !system.is_empty() {
                    body["system"] = json!(system.join("\n\n"));
                }
                ("v1/messages", body, ("x-api-key", s.api_key.clone()))
            }
            LlmProvider::OpenAi => (
                "v1/chat/completions",
                json!({ "model": s.model, "messages": messages, "max_tokens": max_tokens }),
                ("authorization", format!("Bearer {}", s.api_key)),
            ),
        };

        let url = s.base_url.join(path).map_err(|_| LlmError::Transport)?;
        let resp = self
            .http
            .post(url)
            .header("anthropic-version", "2023-06-01")
            .header(header.0, header.1)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout
                } else {
                    tracing::warn!("llm provider unreachable: {}", e.without_url());
                    LlmError::Transport
                }
            })?;

        let status = resp.status();
        if !status.is_success() {
            tracing::warn!(status = status.as_u16(), "llm provider returned an error");
            return Err(LlmError::Provider(status.as_u16()));
        }

        let raw: serde_json::Value = resp.json().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout
            } else {
                LlmError::Malformed
            }
        })?;
        extract_content(&raw, s.provider).ok_or(LlmError::Malformed)
    }
}

fn extract_content(raw: &serde_json::Value, provider: LlmProvider) -> Option<String> {
    let text = match provider {
        LlmProvider::Anthropic => raw["content"][0]["text"].as_str(),
        LlmProvider::OpenAi => raw["choices"][0]["message"]["content"].as_str(),
    }?;
    Some(text.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStatus {
    pub configured: bool,
    pub provider: Option<&'static str>,
    pub model: Option<String>,
}

impl LlmStatus {
    pub fn of(client: Option<&LlmClient>) -> Self {
        match client {
            Some(c) => Self {
                configured: true,
                provider: Some(c.provider().name()),
                model: Some(c.model().to_string()),
            },
            None => Self {
                configured: false,
                provider: None,
                model: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::HeaderMap, routing::post, Router};
    use std::sync::Arc;

    /// A fake provider: `mode` picks the response; the api key must be present.
    #[derive(Clone)]
    struct Fake {
        mode: &'static str,
    }

    async fn anthropic(
        State(f): State<Arc<Fake>>,
        headers: HeaderMap,
        body: String,
    ) -> axum::response::Response {
        if headers.get("x-api-key").is_none() {
            return (StatusCode::UNAUTHORIZED, "no key").into_response();
        }
        let sent: serde_json::Value = serde_json::from_str(&body).unwrap();
        respond(&f.mode, json!({ "content": [{ "type": "text", "text": format!("anthropic:{}", sent["system"]) }] })).await
    }

    async fn openai(State(f): State<Arc<Fake>>, headers: HeaderMap) -> axum::response::Response {
        if headers.get("authorization").is_none() {
            return (StatusCode::UNAUTHORIZED, "no key").into_response();
        }
        respond(
            &f.mode,
            json!({ "choices": [{ "message": { "role": "assistant", "content": "openai:ok" } }] }),
        )
        .await
    }

    async fn respond(mode: &str, ok: serde_json::Value) -> axum::response::Response {
        match mode {
            "ok" => Json(ok).into_response(),
            "error" => (StatusCode::TOO_MANY_REQUESTS, "slow down").into_response(),
            "garbage" => "this is not json".into_response(),
            "wrong-shape" => Json(json!({ "unexpected": true })).into_response(),
            "slow" => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Json(ok).into_response()
            }
            _ => unreachable!(),
        }
    }

    async fn client(provider: LlmProvider, mode: &'static str) -> LlmClient {
        let app = Router::new()
            .route("/v1/messages", post(anthropic))
            .route("/v1/chat/completions", post(openai))
            .with_state(Arc::new(Fake { mode }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        LlmClient::new(LlmSettings {
            provider,
            model: "test-model".into(),
            api_key: "test-key".into(),
            base_url: Url::parse(&format!("http://{addr}/")).unwrap(),
            timeout: Duration::from_millis(300),
        })
        .unwrap()
    }

    fn msgs() -> Vec<LlmMessage> {
        vec![
            LlmMessage {
                role: LlmRole::System,
                content: "be brief".into(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: "hi".into(),
            },
        ]
    }

    #[tokio::test]
    async fn anthropic_success_lifts_system_and_sends_key() {
        let c = client(LlmProvider::Anthropic, "ok").await;
        assert_eq!(
            c.complete(&msgs(), None).await.unwrap(),
            "anthropic:\"be brief\""
        );
    }

    #[tokio::test]
    async fn openai_success_sends_bearer() {
        let c = client(LlmProvider::OpenAi, "ok").await;
        assert_eq!(c.complete(&msgs(), Some(10)).await.unwrap(), "openai:ok");
    }

    #[tokio::test]
    async fn provider_error_status_is_reported_without_body() {
        let c = client(LlmProvider::OpenAi, "error").await;
        let err = c.complete(&msgs(), None).await.unwrap_err();
        assert!(matches!(err, LlmError::Provider(429)), "{err:?}");
        assert!(!err.to_string().contains("slow down"));
    }

    #[tokio::test]
    async fn non_json_body_is_malformed() {
        let c = client(LlmProvider::Anthropic, "garbage").await;
        assert!(matches!(
            c.complete(&msgs(), None).await.unwrap_err(),
            LlmError::Malformed
        ));
    }

    #[tokio::test]
    async fn wrong_shape_is_malformed() {
        let c = client(LlmProvider::OpenAi, "wrong-shape").await;
        assert!(matches!(
            c.complete(&msgs(), None).await.unwrap_err(),
            LlmError::Malformed
        ));
    }

    #[tokio::test]
    async fn slow_provider_times_out() {
        let c = client(LlmProvider::OpenAi, "slow").await;
        assert!(matches!(
            c.complete(&msgs(), None).await.unwrap_err(),
            LlmError::Timeout
        ));
    }

    #[tokio::test]
    async fn unreachable_provider_is_transport() {
        let c = LlmClient::new(LlmSettings {
            provider: LlmProvider::OpenAi,
            model: "m".into(),
            api_key: "k".into(),
            base_url: Url::parse("http://127.0.0.1:1/").unwrap(),
            timeout: Duration::from_millis(300),
        })
        .unwrap();
        assert!(matches!(
            c.complete(&msgs(), None).await.unwrap_err(),
            LlmError::Transport
        ));
    }

    #[tokio::test]
    async fn request_validation_happens_before_any_call() {
        let c = client(LlmProvider::OpenAi, "ok").await;
        assert!(matches!(
            c.complete(&[], None).await.unwrap_err(),
            LlmError::Validation(_)
        ));
        assert!(matches!(
            c.complete(&msgs(), Some(0)).await.unwrap_err(),
            LlmError::Validation(_)
        ));
        assert!(matches!(
            c.complete(&msgs(), Some(MAX_TOKENS_CAP + 1))
                .await
                .unwrap_err(),
            LlmError::Validation(_)
        ));
    }

    #[test]
    fn status_codes() {
        for (err, code) in [
            (LlmError::NotConfigured, 503),
            (LlmError::Validation("x".into()), 422),
            (LlmError::Timeout, 504),
            (LlmError::Transport, 502),
            (LlmError::Provider(500), 502),
            (LlmError::Malformed, 502),
        ] {
            assert_eq!(err.into_response().status().as_u16(), code);
        }
    }

    #[test]
    fn debug_redacts_api_key() {
        let s = LlmSettings {
            provider: LlmProvider::OpenAi,
            model: "m".into(),
            api_key: "super-secret".into(),
            base_url: Url::parse("https://api.openai.com/").unwrap(),
            timeout: Duration::from_secs(1),
        };
        let text = format!("{s:?}");
        assert!(
            !text.contains("super-secret") && text.contains("<redacted>"),
            "{text}"
        );
    }
}
