use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use app::services::artifact::ArtifactStore;
use infra::repositories::artifact::PgArtifactRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

/// Read an owned artifact (#11): metadata, or the exact bytes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/artifacts/{id}", get(get_meta))
        .route("/artifacts/{id}/bytes", get(get_bytes))
}

fn store(state: AppState) -> ArtifactStore<PgArtifactRepository> {
    ArtifactStore::new(PgArtifactRepository::new(state.pool), state.uploads_dir)
}

async fn get_meta(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    Ok(Json(store(state).get(user_id, id).await?))
}

async fn get_bytes(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let (artifact, bytes) = store(state).read(user_id, id).await?;
    let filename = artifact
        .original_filename
        .replace(['"', '\\', '\r', '\n'], "_");
    Ok((
        [
            (header::CONTENT_TYPE, artifact.content_type),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{filename}\""),
            ),
            (header::ETAG, format!("\"{}\"", artifact.sha256)),
        ],
        bytes,
    ))
}
