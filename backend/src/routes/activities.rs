use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use app::dtos::activity::{CreateActivityRequest, RevisionContentRequest};
use app::services::activity::ActivityService;
use infra::repositories::activity::PgActivityRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

/// Authoring and selecting activities (#41). Every handler is owner-scoped
/// through the service; read responses withhold the answer key.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/activities", get(list).post(create))
        .route("/activities/{id}", get(get_one))
        .route("/activities/{id}/revisions", post(revise))
        .route("/revisions/{id}", get(get_revision))
}

fn svc(state: AppState) -> ActivityService<PgActivityRepository> {
    ActivityService::new(PgActivityRepository::new(state.pool))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    Ok(Json(svc(state).list(user_id).await?))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    Ok(Json(svc(state).get(user_id, id).await?))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateActivityRequest>,
) -> Result<impl IntoResponse, HttpError> {
    Ok((
        StatusCode::CREATED,
        Json(svc(state).create(user_id, req).await?),
    ))
}

async fn revise(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RevisionContentRequest>,
) -> Result<impl IntoResponse, HttpError> {
    Ok((
        StatusCode::CREATED,
        Json(svc(state).revise(user_id, id, req).await?),
    ))
}

async fn get_revision(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    Ok(Json(svc(state).get_revision(user_id, id).await?))
}
