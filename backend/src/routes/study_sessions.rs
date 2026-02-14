use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, patch, post},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::study_session::{CreateStudySessionRequest, UpdateProgressRequest};
use app::services::study_session::StudySessionService;
use infra::repositories::study_session::PgStudySessionRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/study-sessions",              get(list).post(create))
        .route("/study-sessions/{id}",          patch(update_progress))
        .route("/study-sessions/{id}/complete", post(complete))
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>, // "active" | "completed"
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let completed = q.status.as_deref().map(|s| s == "completed");
    let svc = StudySessionService::new(PgStudySessionRepository::new(state.pool));
    Ok(Json(svc.list(user_id, completed).await?))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateStudySessionRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = StudySessionService::new(PgStudySessionRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(user_id, req).await?)))
}

async fn update_progress(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = StudySessionService::new(PgStudySessionRepository::new(state.pool));
    Ok(Json(svc.update_progress(id, user_id, req).await?))
}

async fn complete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = StudySessionService::new(PgStudySessionRepository::new(state.pool));
    Ok(Json(svc.complete(id, user_id).await?))
}
