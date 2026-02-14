use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::get,
    Json,
};

use app::dtos::settings::UpdateSettingsRequest;
use app::services::settings::SettingsService;
use infra::repositories::settings::PgSettingsRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/settings", get(get_settings).patch(update_settings))
}

async fn get_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    let svc = SettingsService::new(PgSettingsRepository::new(state.pool));
    Ok(Json(svc.get(user_id).await?))
}

async fn update_settings(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = SettingsService::new(PgSettingsRepository::new(state.pool));
    Ok(Json(svc.update(user_id, req).await?))
}
