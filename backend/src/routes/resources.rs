use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    response::IntoResponse,
    routing::{delete, get},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use app::dtos::resource::CreateResourceRequest;
use app::errors::AppError;
use app::services::resource::ResourceService;
use app::services::upload::UploadService;
use domain::repository_traits::ResourceRepository;
use infra::repositories::resource::PgResourceRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/resources",              get(list).post(create))
        .route("/resources/upload",       axum::routing::post(upload)
            .layer(DefaultBodyLimit::max(50 * 1024 * 1024))) // 50 MB
        .route("/resources/{id}",         delete(delete_one))
        .route("/resources/{id}/content", get(get_content))
        .route("/resources/{id}/pages",   get(get_pages))
        .route("/resources/{id}/file",    get(get_file))
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

async fn upload(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, HttpError> {
    let uploads_dir = std::path::PathBuf::from(
        std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "data/uploads".to_string()),
    );

    let mut filename = String::new();
    let mut bytes = Vec::new();
    let mut topic_id_str = String::new();
    let mut title = String::new();
    let mut concept_ids_json = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| HttpError(AppError::Validation(format!("multipart error: {e}"))))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = field.file_name().unwrap_or("upload").to_string();
                bytes = field
                    .bytes()
                    .await
                    .map_err(|e| HttpError(AppError::Unexpected(format!("read file: {e}"))))?
                    .to_vec();
            }
            "topicId" => topic_id_str = field.text().await.unwrap_or_default(),
            "title" => title = field.text().await.unwrap_or_default(),
            "conceptIds" => concept_ids_json = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    if bytes.is_empty() {
        return Err(HttpError(AppError::Validation("file is required".into())));
    }

    let topic_id = Uuid::parse_str(&topic_id_str)
        .map_err(|_| HttpError(AppError::Validation("invalid topicId".into())))?;

    let concept_ids: Vec<String> =
        serde_json::from_str(&concept_ids_json).unwrap_or_default();
    let concept_uuids = concept_ids
        .iter()
        .map(|s| Uuid::parse_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| HttpError(AppError::Validation("invalid conceptId".into())))?;

    // Extract text based on file type
    let is_pdf = filename.to_lowercase().ends_with(".pdf");
    let resource_type = if is_pdf { "pdf" } else { "article" };
    let content_pages = if is_pdf {
        infra::pdf::extract_text_by_pages(&bytes)
    } else {
        vec![String::from_utf8_lossy(&bytes).to_string()]
    };
    let content_text = content_pages.join("\n");

    if title.is_empty() {
        title = filename
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(&filename)
            .to_string();
    }

    let svc = UploadService::new(PgResourceRepository::new(state.pool), uploads_dir);
    let resp = svc
        .upload(user_id, topic_id, title, filename, &bytes, content_text, content_pages, resource_type, concept_uuids)
        .await?;

    Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

async fn get_content(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = UploadService::new(
        PgResourceRepository::new(state.pool),
        std::path::PathBuf::from("data/uploads"), // not used for get_content
    );
    let text = svc.get_content(id, user_id).await?;
    Ok(Json(serde_json::json!({ "text": text })))
}

#[derive(Deserialize)]
struct PagesQuery {
    #[serde(default)]
    start: usize,
    #[serde(default = "default_page_end")]
    end: usize,
}
fn default_page_end() -> usize { usize::MAX }

async fn get_pages(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<PagesQuery>,
) -> Result<impl IntoResponse, HttpError> {
    let repo = PgResourceRepository::new(state.pool);
    let pages = repo.get_pages(id, user_id, q.start, q.end).await
        .map_err(HttpError::from)?;
    Ok(Json(serde_json::json!({ "pages": pages })))
}

async fn get_file(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let repo = PgResourceRepository::new(state.pool);
    let resource = repo.find_by_id(id, user_id).await.map_err(HttpError::from)?;

    let file_path = resource
        .file_path
        .ok_or_else(|| HttpError(AppError::Validation("resource has no file".into())))?;

    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| HttpError(AppError::Unexpected(format!("read file: {e}"))))?;

    let content_type = if file_path.ends_with(".pdf") {
        "application/pdf"
    } else if file_path.ends_with(".md") {
        "text/markdown"
    } else {
        "text/plain"
    };

    Ok((
        [(axum::http::header::CONTENT_TYPE, content_type)],
        bytes,
    ))
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
