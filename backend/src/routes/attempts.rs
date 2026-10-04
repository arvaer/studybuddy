use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use app::dtos::attempt::RecordAttemptRequest;
use app::services::attempt::AttemptService;
use infra::repositories::attempt::PgAttemptRepository;
use infra::repositories::workspace::PgWorkspaceRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/attempts", post(record))
        .route("/attempts/{id}", get(get_one))
}

/// Record an attempt against an owned activity revision (#9). A resent
/// request key replays the earlier receipt with 200 instead of 201 (#10).
async fn record(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<RecordAttemptRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = AttemptService::new(PgAttemptRepository::new(state.pool.clone()));
    let recorded = svc.record(user_id, req).await?;
    let status = if recorded.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    // An answer wakes the operator (20b): if the learner's workspace has a
    // wait parked on this revision, the attempt is its receipt and the run
    // goes on in the background. The attempt stands whatever happens here.
    if let Ok(Some(workspace)) = PgWorkspaceRepository::new(state.pool.clone())
        .find(user_id)
        .await
    {
        if let Err(error) = state.runtime.poke(workspace).await {
            tracing::warn!(workspace = %workspace, "could not wake the operator: {error}");
        }
    }
    Ok((status, Json(recorded.receipt)))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = AttemptService::new(PgAttemptRepository::new(state.pool));
    Ok(Json(svc.get(user_id, id).await?))
}
