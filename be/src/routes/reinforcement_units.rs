use axum::{
    Router,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::reinforcement_unit::{CreateRuRequest, UpdateRuRequest};
use app::services::reinforcement_unit::RuService;
use infra::repositories::reinforcement_unit::PgRuRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/reinforcement-units",     get(list).post(create))
        .route("/reinforcement-units/{id}", get(get_one).patch(update))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "conceptId")]
    concept_id: Option<Uuid>,
    state: Option<String>,
}

async fn create(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Json(req): Json<CreateRuRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = RuService::new(PgRuRepository::new(state.pool));
    Ok((axum::http::StatusCode::CREATED, Json(svc.create(req).await?)))
}

async fn list(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = RuService::new(PgRuRepository::new(state.pool));
    Ok(Json(svc.list(q.concept_id, q.state.as_deref()).await?))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = RuService::new(PgRuRepository::new(state.pool));
    Ok(Json(svc.get(id).await?))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRuRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = RuService::new(PgRuRepository::new(state.pool));
    Ok(Json(svc.update_after_review(id, req).await?))
}
