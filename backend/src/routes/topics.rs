use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use uuid::Uuid;

use app::dtos::topic::{CreateTopicRequest, UpdateTopicRequest};
use app::services::topic::TopicService;
use infra::repositories::topic::PgTopicRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/topics",          get(list).post(create))
        .route("/topics/{id}",      get(get_one).patch(update).delete(delete_one))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    let svc = TopicService::new(PgTopicRepository::new(state.pool));
    Ok(Json(svc.list(user_id).await?))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = TopicService::new(PgTopicRepository::new(state.pool));
    Ok(Json(svc.get(id, user_id).await?))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateTopicRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = TopicService::new(PgTopicRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(user_id, req).await?)))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTopicRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = TopicService::new(PgTopicRepository::new(state.pool));
    Ok(Json(svc.update(id, user_id, req).await?))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = TopicService::new(PgTopicRepository::new(state.pool));
    svc.delete(id, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
