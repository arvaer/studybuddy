use axum::{extract::State, routing::{get, post}, Json, Router};

use app::dtos::llm::{LlmProxyRequest, LlmProxyResponse};

use crate::llm::{LlmError, LlmStatus};
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/llm/proxy", post(proxy))
        .route("/llm/status", get(status))
}

/// Forward messages to the server-configured provider. The request cannot
/// name a provider, model, key or URL; see `LlmProxyRequest`.
async fn proxy(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Json(req): Json<LlmProxyRequest>,
) -> Result<Json<LlmProxyResponse>, LlmError> {
    let client = state.llm.as_ref().ok_or(LlmError::NotConfigured)?;
    let content = client.complete(&req.messages, req.max_tokens).await?;
    Ok(Json(LlmProxyResponse { content }))
}

/// Whether this server has a provider, and which model. No credential.
async fn status(State(state): State<AppState>, AuthUser(_user_id): AuthUser) -> Json<LlmStatus> {
    Json(LlmStatus::of(state.llm.as_deref()))
}
