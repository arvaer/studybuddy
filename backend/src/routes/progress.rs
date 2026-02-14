use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::get,
    Json,
};

use app::services::progress::ProgressService;
use infra::repositories::progress::PgProgressRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/progress", get(get_progress))
}

async fn get_progress(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ProgressService::new(PgProgressRepository::new(state.pool));
    Ok(Json(svc.get(user_id).await?))
}
