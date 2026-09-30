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
    let svc = AttemptService::new(PgAttemptRepository::new(state.pool));
    let recorded = svc.record(user_id, req).await?;
    let status = if recorded.replayed { StatusCode::OK } else { StatusCode::CREATED };
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
