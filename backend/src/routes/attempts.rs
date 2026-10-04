use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

use app::dtos::attempt::RecordAttemptRequest;
use app::services::attempt::AttemptService;
use infra::repositories::attempt::PgAttemptRepository;
use infra::repositories::workspace::PgWorkspaceRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/attempts", post(record))
        .route("/attempts/{id}", get(get_one))
        .route(
            "/attempts/drafts/{revision}/hint",
            post(request_hint).get(hints),
        )
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HintRequest {
    /// The learner's draft so far, for the coach to read.
    #[serde(default)]
    pub draft: String,
}

/// Ask the operator for a hint on an owned revision (20c). The request is
/// recorded and the operator poked: its wait on this revision is answered
/// with the request, and the hint lands on `GET` the same path. 202.
async fn request_hint(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(revision): Path<Uuid>,
    body: Option<Json<HintRequest>>,
) -> Result<impl IntoResponse, HttpError> {
    let Json(req) = body.unwrap_or_default();
    let workspace = PgWorkspaceRepository::new(state.pool.clone())
        .find_or_create(user_id)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO hint_requests (workspace_id, activity_revision_id, draft)
         SELECT $1, r.id, $3 FROM activity_revisions r JOIN activities a ON a.id = r.activity_id
         WHERE r.id = $2 AND a.user_id = $4",
    )
    .bind(workspace)
    .bind(revision)
    .bind(req.draft.chars().take(4000).collect::<String>())
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| HttpError(domain::errors::DomainError::Repository(e.to_string()).into()))?;
    if inserted.rows_affected() == 0 {
        return Err(HttpError(
            domain::errors::DomainError::NotFound(format!("revision {revision}")).into(),
        ));
    }
    if let Err(error) = state.runtime.poke(workspace).await {
        tracing::warn!(workspace = %workspace, "could not wake the operator for a hint: {error}");
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "requested": true })),
    ))
}

/// The hints the operator has given on an owned revision, oldest first.
async fn hints(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(revision): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT h.id, h.text, to_char(h.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') FROM hints h
         JOIN activity_revisions r ON r.id = h.activity_revision_id
         JOIN activities a ON a.id = r.activity_id
         WHERE h.activity_revision_id = $1 AND a.user_id = $2
         ORDER BY h.created_at",
    )
    .bind(revision)
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| HttpError(domain::errors::DomainError::Repository(e.to_string()).into()))?;
    let hints: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, text, at)| serde_json::json!({ "id": id, "text": text, "createdAt": at }))
        .collect();
    Ok(Json(serde_json::json!({ "hints": hints })))
}

/// Record an attempt against an owned activity revision (#9). A resent
/// request key replays the earlier receipt with 200 instead of 201 (#10).
async fn record(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<RecordAttemptRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = AttemptService::new(PgAttemptRepository::new(state.pool.clone()));
    let recorded = svc.record(user_id, req).await?;
    let status = if recorded.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    // An answer wakes the operator (20b): if the learner's workspace has a
    // wait parked on this revision, the attempt is its receipt and the run
    // goes on in the background. The attempt stands whatever happens here.
    if let Ok(Some(workspace)) = PgWorkspaceRepository::new(state.pool.clone())
        .find(user_id)
        .await
    {
        if let Err(error) = state.runtime.poke(workspace).await {
            tracing::warn!(workspace = %workspace, "could not wake the operator: {error}");
        }
    }
    Ok((status, Json(recorded.receipt)))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = AttemptService::new(PgAttemptRepository::new(state.pool));
    Ok(Json(svc.get(user_id, id).await?))
}
