use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, patch},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::note::{CreateNoteRequest, UpdateNoteRequest};
use app::services::note::NoteService;
use infra::repositories::note::PgNoteRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notes",     get(list).post(create))
        .route("/notes/{id}", patch(update).delete(delete_one))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "conceptId")]
    concept_id: Option<Uuid>,
    #[serde(rename = "ruId")]
    ru_id: Option<Uuid>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = NoteService::new(PgNoteRepository::new(state.pool));
    Ok(Json(svc.list(user_id, q.concept_id, q.ru_id).await?))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateNoteRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = NoteService::new(PgNoteRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(user_id, req).await?)))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNoteRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = NoteService::new(PgNoteRepository::new(state.pool));
    Ok(Json(svc.update(id, user_id, req).await?))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = NoteService::new(PgNoteRepository::new(state.pool));
    svc.delete(id, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
