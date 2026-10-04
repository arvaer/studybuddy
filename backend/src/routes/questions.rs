use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use app::services::question::QuestionService;
use infra::repositories::question::PgQuestionRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/questions", get(list))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(rename = "ruId")]
    ru_id: Option<Uuid>,
    #[serde(rename = "conceptId")]
    concept_id: Option<Uuid>,
    #[serde(rename = "topicId")]
    topic_id: Option<Uuid>,
    #[serde(rename = "type")]
    question_type: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = QuestionService::new(PgQuestionRepository::new(state.pool));
    Ok(Json(
        svc.list(
            user_id,
            q.ru_id,
            q.concept_id,
            q.topic_id,
            q.question_type.as_deref(),
        )
        .await?,
    ))
}
