use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use uuid::Uuid;

use app::dtos::quiz::{CreateQuizSessionRequest, SubmitAnswerRequest};
use app::services::quiz::QuizService;
use infra::repositories::quiz_session::PgQuizSessionRepository;
use infra::repositories::reinforcement_unit::PgRuRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/quiz-sessions",                       post(create))
        .route("/quiz-sessions/{id}",                   get(get_one))
        .route("/quiz-sessions/{id}/submit",            post(submit))
        .route("/quiz-sessions/{id}/complete",          post(complete))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateQuizSessionRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    Ok((axum::http::StatusCode::CREATED, Json(svc.create_session(user_id, req).await?)))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    Ok(Json(svc.get_session(id, user_id).await?))
}

async fn submit(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SubmitAnswerRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    Ok(Json(svc.submit_answer(id, user_id, req).await?))
}

async fn complete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    Ok(Json(svc.complete_session(id, user_id).await?))
}

fn make_service(state: &AppState) -> QuizService<PgQuizSessionRepository, PgRuRepository> {
    QuizService::new(
        PgQuizSessionRepository::new(state.pool.clone()),
        PgRuRepository::new(state.pool.clone()),
    )
}
