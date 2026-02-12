use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::concept::{CreateConceptRequest, UpdateConceptRequest};
use app::services::concept::ConceptService;
use infra::repositories::concept::PgConceptRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/concepts",     get(list).post(create))
        .route("/concepts/{id}", get(get_one).patch(update).delete(delete_one))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "topicId")]
    topic_id: Option<Uuid>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ConceptService::new(PgConceptRepository::new(state.pool));
    Ok(Json(svc.list(user_id, q.topic_id).await?))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ConceptService::new(PgConceptRepository::new(state.pool));
    Ok(Json(svc.get(id, user_id).await?))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateConceptRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ConceptService::new(PgConceptRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(user_id, req).await?)))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateConceptRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ConceptService::new(PgConceptRepository::new(state.pool));
    Ok(Json(svc.update(id, user_id, req).await?))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ConceptService::new(PgConceptRepository::new(state.pool));
    svc.delete(id, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
