use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;

use app::services::rate_limit::AuthLimiter;

use crate::llm::LlmClient;
use crate::runtime::OperatorRuntime;

#[derive(Clone)]
pub struct AppState {
    pub pool:        PgPool,
    pub jwt_secret:  String,
    pub uploads_dir: PathBuf,
    /// Set the `Secure` attribute on auth cookies (`COOKIE_SECURE`).
    pub cookie_secure: bool,
    /// `None` when no `LLM_*` configuration was given; the proxy answers 503.
    pub llm:         Option<Arc<LlmClient>>,
    /// Login/signup attempt limiter (`AUTH_RATE_LIMIT_*`), in-process (#37).
    pub auth_limiter: Arc<AuthLimiter>,
    /// `None` when `OPERATOR_SECRET` is unset; `/internal/effects` answers 404.
    pub operator_secret: Option<Arc<str>>,
    /// The operator's model (19b): upstream's Claude adapter, installed on
    /// each session the owner opens. `None` without `ANTHROPIC_API_KEY`.
    pub operator_model: Option<operator::Model>,
    /// This process's sessions and runs (20a).
    pub runtime: Arc<OperatorRuntime>,
}
