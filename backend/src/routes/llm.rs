use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::post,
    Json,
};

use app::dtos::llm::{LlmProxyRequest, LlmProxyResponse};
use app::errors::AppError;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/llm/proxy", post(proxy))
}

async fn proxy(
    State(_state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Json(req): Json<LlmProxyRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if req.api_key.trim().is_empty() {
        return Err(HttpError(AppError::Validation("api_key is required".into())));
    }

    let client = reqwest::Client::new();

    let base_url = req.base_url.trim_end_matches('/');
    let max_tokens = req.max_tokens.unwrap_or(2048);

    let (url, request_body, headers) = match req.provider.as_str() {
        "anthropic" => {
            let url = format!("{base_url}/v1/messages");

            // Anthropic expects system pulled out of messages array
            let system = req
                .messages
                .iter()
                .find(|m| m["role"] == "system")
                .and_then(|m| m["content"].as_str())
                .unwrap_or("");
            let user_messages: Vec<_> = req
                .messages
                .iter()
                .filter(|m| m["role"] != "system")
                .collect();

            let body = serde_json::json!({
                "model": req.model,
                "system": system,
                "messages": user_messages,
                "max_tokens": max_tokens,
            });

            let mut h = vec![
                ("x-api-key".to_string(), req.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ];
            (url, body, h)
        }
        _ => {
            // OpenAI-compatible (covers OpenAI, Ollama, etc.)
            let url = format!("{base_url}/v1/chat/completions");
            let body = serde_json::json!({
                "model": req.model,
                "messages": req.messages,
                "max_tokens": max_tokens,
            });
            let h = vec![
                ("Authorization".to_string(), format!("Bearer {}", req.api_key)),
            ];
            (url, body, h)
        }
    };

    let mut builder = client
        .post(&url)
        .header("Content-Type", "application/json");

    for (key, value) in &headers {
        builder = builder.header(key, value);
    }

    let resp = builder
        .json(&request_body)
        .send()
        .await
        .map_err(|e| HttpError(AppError::Unexpected(format!("LLM request failed: {e}"))))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(HttpError(AppError::Unexpected(format!(
            "LLM provider error {status}: {text}"
        ))));
    }

    let raw: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| HttpError(AppError::Unexpected(format!("LLM response parse error: {e}"))))?;

    let content = extract_content(&raw, &req.provider)
        .ok_or_else(|| HttpError(AppError::Unexpected("empty LLM response".into())))?;

    Ok(Json(LlmProxyResponse { content, raw }))
}

fn extract_content(raw: &serde_json::Value, provider: &str) -> Option<String> {
    if provider == "anthropic" {
        raw["content"][0]["text"].as_str().map(String::from)
    } else {
        raw["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
    }
}
