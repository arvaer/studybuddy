use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::resource::CreateResourceRequest;
use app::services::resource::ResourceService;
use infra::repositories::resource::PgResourceRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/resources",     get(list).post(create))
        .route("/resources/{id}", delete(delete_one))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "topicId")]
    topic_id: Option<Uuid>,
    #[serde(rename = "conceptId")]
    concept_id: Option<Uuid>,
    #[serde(rename = "type")]
    resource_type: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ResourceService::new(PgResourceRepository::new(state.pool));
    Ok(Json(
        svc.list(user_id, q.topic_id, q.concept_id, q.resource_type.as_deref())
            .await?,
    ))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateResourceRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ResourceService::new(PgResourceRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(user_id, req).await?)))
}

async fn delete_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = ResourceService::new(PgResourceRepository::new(state.pool));
    svc.delete(id, user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
