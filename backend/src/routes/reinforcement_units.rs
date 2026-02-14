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
use app::services::event::EventService;
use app::services::reinforcement_unit::RuService;
use infra::repositories::event::PgEventRepository;
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
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateRuRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let concept_id = Uuid::parse_str(&req.concept_id)
        .map_err(|_| HttpError(app::errors::AppError::Validation("invalid concept_id".into())))?;

    let svc = RuService::new(PgRuRepository::new(state.pool.clone()));
    let created = svc.create(req).await?;

    // Append event for each created RU (fire-and-forget)
    let event_svc = EventService::new(PgEventRepository::new(state.pool.clone()));
    for ru in &created {
        if let Ok(ru_id) = Uuid::parse_str(&ru.id) {
            let payload = serde_json::json!({
                "claim": ru.claim,
                "context": ru.context,
                "conceptId": concept_id.to_string(),
                "sourceResourceId": ru.source_resource_id,
            });
            if let Err(e) = event_svc
                .append_event(
                    user_id,
                    "knowledge.claim_introduced.v1",
                    "reinforcement_unit",
                    ru_id,
                    payload,
                )
                .await
            {
                tracing::warn!("Event append failed for RU {}: {e}", ru.id);
            }
        }
    }

    Ok((axum::http::StatusCode::CREATED, Json(created)))
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
