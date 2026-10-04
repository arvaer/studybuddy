//! The operator's providers as HTTP endpoints (19a; docs/phase-2-build.md,
//! "The one architectural choice"). The request and the reply are upstream's
//! connector protocol (capsule-corp sdk-surface, "The connector protocol",
//! L1c): the request is the effect as JSON, `{"id", "capability",
//! "payload"}`, the id doubles as `Idempotency-Key`, and the reply is one
//! object, `{"value"}`, `{"refused"}`, `{"declined"}` or `{"unknown"}`, in a
//! 2xx body. Any other status reads as `unknown` to the connector, which is
//! the right reading for "the effect may have committed" (H3).
//!
//! Never reachable from the browser: the bearer secret is the process's
//! (`OPERATOR_SECRET`), not a learner's token. Bound to one workspace by the
//! URL, which the session's manifest fixes; and the payload's scope path
//! must lie inside that workspace, so a misbound manifest cannot publish
//! into another learner's workspace even where the environment let the
//! capsule name the path.
//!
//! Two families: `learning/present` at `learning.present`, whose reply and
//! receipt are the same JSON the manual route answers, produced by the same
//! `ActivityService::create` on one transaction with the receipt; and
//! `learning/assess` at `learning.assess` (20f), the operator's assessment
//! of an attempt, written as the attempt's next `assessments` revision with
//! method `model` on one transaction with its receipt. Both land or
//! neither does, and a replayed id answers the receipt and writes nothing.
//! An assessment that names an attempt this learner does not have answers
//! a value saying so rather than a refusal, so a slip by the model does not
//! end the run. A third, `source/read` at `source.read` (21a), is a
//! read-only look at pages of the learner's own sources: no receipt, a
//! page cap, and `{"found": false}` for an id that is not theirs.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use app::dtos::activity::{CreateActivityRequest, RevisionContentRequest};
use app::errors::AppError;
use app::services::activity::ActivityService;
use domain::errors::DomainError;
use infra::repositories::activity::PgActivityRepository;
use operator::receipts::{self, Receipt, Recorded};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/internal/effects/{workspace}/{family}", post(effect))
}

/// The family in the URL, where a slash cannot go, and as the capsule
/// applies it, which is what the request's `capability` must say.
const PRESENT_PATH: &str = "learning.present";
const PRESENT_FAMILY: &str = "learning/present";
const ASSESS_PATH: &str = "learning.assess";
const ASSESS_FAMILY: &str = "learning/assess";
const SOURCE_PATH: &str = "source.read";
const SOURCE_FAMILY: &str = "source/read";
/// The most pages one read answers, and the most characters of each.
const READ_MAX_PAGES: usize = 4;
const READ_MAX_CHARS: usize = 4000;

/// The effect as the connector posts it.
#[derive(Debug, Deserialize)]
struct EffectRequest {
    id: String,
    capability: String,
    payload: Vec<Value>,
}

fn value(v: Value) -> Response {
    Json(json!({ "value": v })).into_response()
}

/// Nothing was performed and asking again would not help; the run sees
/// `Outcome::Refused`. No receipt is written, so a corrected program can
/// perform under a fresh id.
fn refused(why: impl Into<String>) -> Response {
    Json(json!({ "refused": why.into() })).into_response()
}

/// A failure the endpoint cannot classify. The connector reads a 500 as
/// `unknown` and parks the effect uncertain, which is correct: the
/// transaction may or may not have committed.
fn database(e: sqlx::Error) -> Response {
    tracing::error!("effect endpoint database error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

fn application(e: AppError) -> Response {
    match e {
        AppError::Domain(DomainError::Repository(msg)) => {
            tracing::error!("effect endpoint repository error: {msg}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        // Validation, ownership (`NotFound`), conflicts: deterministic, and
        // nothing was written. The reason, without the error type's prefix.
        AppError::Domain(DomainError::NotFound(what)) => refused(format!("not found: {what}")),
        AppError::Domain(DomainError::Validation(why) | DomainError::Conflict(why))
        | AppError::Validation(why)
        | AppError::Conflict(why)
        | AppError::Unauthorized(why) => refused(why),
        other => refused(other.to_string()),
    }
}

fn authorized(headers: &HeaderMap, secret: &str) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|given| constant_time_eq(given.as_bytes(), secret.as_bytes()))
}

/// Equal length and bytes, examined in full whatever the input.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

async fn effect(
    State(state): State<AppState>,
    Path((workspace, family)): Path<(Uuid, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Authorization before the body is even parsed.
    let Some(secret) = state.operator_secret.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !authorized(&headers, secret) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response();
    }
    let served = match family.as_str() {
        PRESENT_PATH => PRESENT_FAMILY,
        ASSESS_PATH => ASSESS_FAMILY,
        SOURCE_PATH => SOURCE_FAMILY,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let req: EffectRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "body is not an effect" })),
            )
                .into_response();
        }
    };
    if req.capability != served {
        return refused(format!("{family} serves {served}, not {}", req.capability));
    }
    if let Some(key) = headers.get("idempotency-key").and_then(|v| v.to_str().ok()) {
        if key != req.id {
            return refused("Idempotency-Key differs from the effect id");
        }
    }

    let owner = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM workspaces WHERE id = $1")
        .bind(workspace)
        .fetch_optional(&state.pool)
        .await;
    let owner = match owner {
        Ok(Some(owner)) => owner,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return database(e),
    };

    let served = match served {
        PRESENT_FAMILY => present(&state, workspace, owner, &req).await,
        ASSESS_FAMILY => assess(&state, workspace, owner, &req).await,
        _ => read(&state, workspace, owner, &req).await,
    };
    match served {
        Ok(reply) | Err(reply) => reply,
    }
}

/// The scope path as the capsule spells it, checked against the workspace
/// in the URL: a string, or the segment list the kernel read it from.
fn scoped_path(workspace: Uuid, path: &Value) -> Result<String, Response> {
    let path = match path {
        Value::String(path) => path.clone(),
        Value::Array(segments) => segments
            .iter()
            .map(|s| s.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| refused("path segments must be strings"))?
            .join("/"),
        _ => return Err(refused("path must be a string or a list of segments")),
    };
    let scope = format!("workspaces/{workspace}/");
    if !path.starts_with(&scope) {
        return Err(refused(format!("path is outside {scope}*")));
    }
    Ok(path)
}

/// `[path, attempt-id, outcome, feedback]` (`coach/assess`). Shape errors
/// are the program's and refuse; a wrong attempt id or outcome word is the
/// model's and is answered below, not refused.
fn parse_assess(workspace: Uuid, payload: &[Value]) -> Result<(String, String, String), Response> {
    let [path, attempt, outcome, feedback] = payload else {
        return Err(refused(format!(
            "{ASSESS_FAMILY} takes [path attempt-id outcome feedback]"
        )));
    };
    scoped_path(workspace, path)?;
    let Some(attempt) = attempt.as_str() else {
        return Err(refused("attempt-id must be a string"));
    };
    let Some(outcome) = outcome.as_str() else {
        return Err(refused("outcome must be a string"));
    };
    let Some(feedback) = feedback.as_str() else {
        return Err(refused("feedback must be a string"));
    };
    Ok((
        attempt.to_string(),
        outcome.to_string(),
        feedback.to_string(),
    ))
}

/// `[path, resource-id, from, to]` (`coach/read`): pages `from..=to`,
/// 1-based, of one of the learner's sources. Read-only, so no receipt: a
/// crash mid-read is served again under the same id. At most
/// `READ_MAX_PAGES` pages of `READ_MAX_CHARS` each, so one read stays a
/// few thousand tokens in the turns that follow. A resource that is not
/// this learner's, or has no page text, answers `{"found": false}`.
async fn read(
    state: &AppState,
    workspace: Uuid,
    owner: Uuid,
    req: &EffectRequest,
) -> Result<Response, Response> {
    let [path, resource, from, to] = req.payload.as_slice() else {
        return Err(refused(format!(
            "{SOURCE_FAMILY} takes [path resource-id from to]"
        )));
    };
    scoped_path(workspace, path)?;
    let not_found = |why: String| Ok(value(json!({ "found": false, "why": why })));
    let Some(resource) = resource.as_str().and_then(|r| r.parse::<Uuid>().ok()) else {
        return not_found(format!("{resource} is not a resource id"));
    };
    let (Some(from), Some(to)) = (from.as_u64(), to.as_u64()) else {
        return Err(refused("from and to must be page numbers, 1-based"));
    };
    let row: Option<(String, Option<Value>)> =
        sqlx::query_as("SELECT title, content_pages FROM resources WHERE id = $1 AND user_id = $2")
            .bind(resource)
            .bind(owner)
            .fetch_optional(&state.pool)
            .await
            .map_err(database)?;
    let Some((title, pages)) = row else {
        return not_found(format!("no source {resource} for this learner"));
    };
    let pages: Vec<String> = pages
        .and_then(|p| serde_json::from_value(p).ok())
        .unwrap_or_default();
    if pages.is_empty() {
        return not_found(format!("{title} has no page text"));
    }
    let count = pages.len();
    let from = (from.max(1) as usize).min(count);
    let to = (to as usize)
        .clamp(from, count)
        .min(from + READ_MAX_PAGES - 1);
    let read: Vec<Value> = (from..=to)
        .map(|n| {
            let text = &pages[n - 1];
            let text = match text.char_indices().nth(READ_MAX_CHARS) {
                Some((cut, _)) => format!("{}…", &text[..cut]),
                None => text.clone(),
            };
            json!({ "page": n, "text": text })
        })
        .collect();
    Ok(value(json!({
        "found": true,
        "resourceId": resource,
        "title": title,
        "pageCount": count,
        "from": from,
        "to": to,
        "pages": read,
    })))
}

/// The operator's assessment of an attempt, on one transaction with its
/// receipt. The reply is `{"assessed": true, ...}` with what was written,
/// or `{"assessed": false, "why"}` when the attempt or the outcome word is
/// not one this learner has, so the coach reads it and goes on.
async fn assess(
    state: &AppState,
    workspace: Uuid,
    owner: Uuid,
    req: &EffectRequest,
) -> Result<Response, Response> {
    let mut tx = state.pool.begin().await.map_err(database)?;
    if let Some(receipt) = receipts::find(&mut *tx, workspace, &req.id)
        .await
        .map_err(database)?
    {
        return Ok(value(receipt.payload));
    }
    let (attempt, outcome_word, feedback) = parse_assess(workspace, &req.payload)?;

    let not_assessed = |why: String| json!({ "assessed": false, "why": why });
    let payload = match (
        attempt.parse::<Uuid>().ok(),
        domain::learning::AssessmentOutcome::parse(&outcome_word),
    ) {
        (None, _) => not_assessed(format!("{attempt} is not an attempt id")),
        (_, None) => not_assessed(format!(
            "outcome must be correct, partial or incorrect, not {outcome_word}"
        )),
        (Some(attempt_id), Some(outcome)) => {
            // The attempt must be this learner's; an activity the operator
            // published in this workspace is, and so is any other of theirs.
            let owned: Option<Uuid> =
                sqlx::query_scalar("SELECT id FROM attempts WHERE id = $1 AND user_id = $2")
                    .bind(attempt_id)
                    .bind(owner)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(database)?;
            match owned {
                None => not_assessed(format!("no attempt {attempt_id} for this learner")),
                Some(_) => {
                    let score = match outcome {
                        domain::learning::AssessmentOutcome::Correct => 1.0,
                        domain::learning::AssessmentOutcome::Partial => 0.5,
                        domain::learning::AssessmentOutcome::Incorrect => 0.0,
                    };
                    sqlx::query(
                        "INSERT INTO assessments (attempt_id, revision, outcome, method, score, feedback)
                         SELECT $1, coalesce(max(revision), 0) + 1, ($2::text)::assessment_outcome,
                                'model'::assessment_method, $3, $4
                         FROM assessments WHERE attempt_id = $1",
                    )
                    .bind(attempt_id)
                    .bind(outcome.as_str())
                    .bind(score)
                    .bind(&feedback)
                    .execute(&mut *tx)
                    .await
                    .map_err(database)?;
                    json!({
                        "assessed": true,
                        "attemptId": attempt_id,
                        "status": outcome.as_str(),
                        "method": "model",
                        "feedback": feedback,
                    })
                }
            }
        }
    };

    let receipt = Receipt {
        effect_id: req.id.clone(),
        workspace_id: workspace,
        family: ASSESS_FAMILY.to_string(),
        payload,
    };
    match receipts::record(&mut tx, &receipt)
        .await
        .map_err(database)?
    {
        Recorded::New => {
            tx.commit().await.map_err(database)?;
            Ok(value(receipt.payload))
        }
        Recorded::Existing(earlier) => Ok(value(earlier.payload)),
    }
}

/// `[path, kind, prompt, answer-key]`, as the learning capsule spells it
/// (`backend/capsules/learning.capsule`, `coach/present`): the scope path
/// the environment checked, a string or the segment list the kernel read it
/// from; then the activity's kind, its prompt and the answer the model
/// expects (`null` for none). Capsule source has lists and strings, not
/// objects, so the request is built here, where the manual route's DTO is.
fn parse_present(workspace: Uuid, payload: &[Value]) -> Result<CreateActivityRequest, Response> {
    let [path, kind, prompt, answer_key] = payload else {
        return Err(refused(format!(
            "{PRESENT_FAMILY} takes [path kind prompt answer-key]"
        )));
    };
    scoped_path(workspace, path)?;
    let Some(kind) = kind.as_str() else {
        return Err(refused("kind must be a string"));
    };
    let Some(prompt) = prompt.as_str() else {
        return Err(refused("prompt must be a string"));
    };
    Ok(CreateActivityRequest {
        kind: kind.to_string(),
        concept_id: None,
        revision: RevisionContentRequest {
            prompt: prompt.to_string(),
            options: None,
            answer_key: (!answer_key.is_null()).then(|| answer_key.clone()),
            rubric: None,
            source_resource_id: None,
            source_location: None,
        },
    })
}

/// Everything after authorization, on one transaction: look up the receipt,
/// else perform through the service and write the receipt, then commit.
async fn present(
    state: &AppState,
    workspace: Uuid,
    owner: Uuid,
    req: &EffectRequest,
) -> Result<Response, Response> {
    let mut tx = state.pool.begin().await.map_err(database)?;

    if let Some(receipt) = receipts::find(&mut *tx, workspace, &req.id)
        .await
        .map_err(database)?
    {
        // Already performed: the same observation, nothing written.
        return Ok(value(receipt.payload));
    }

    let activity = parse_present(workspace, &req.payload)?;
    let created = {
        let svc = ActivityService::new(PgActivityRepository::held(&mut tx));
        svc.create(owner, activity).await.map_err(application)?
    };
    let payload = serde_json::to_value(&created).map_err(|e| {
        tracing::error!("effect endpoint could not encode the reply: {e}");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;

    let receipt = Receipt {
        effect_id: req.id.clone(),
        workspace_id: workspace,
        family: PRESENT_FAMILY.to_string(),
        payload,
    };
    match receipts::record(&mut tx, &receipt)
        .await
        .map_err(database)?
    {
        Recorded::New => {
            tx.commit().await.map_err(database)?;
            Ok(value(receipt.payload))
        }
        // Lost a race with the same id: the earlier observation stands and
        // this transaction rolls back with the duplicate activity in it.
        Recorded::Existing(earlier) => Ok(value(earlier.payload)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::dtos::auth::TokenClaims;
    use app::services::rate_limit::{AuthLimiter, AuthLimits};
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use std::sync::Arc;
    use tower::ServiceExt;

    const SECRET: &str = "operator-secret-for-tests";
    const JWT: &str = "jwt-secret-for-tests";

    /// A runtime for tests: no model, the endpoint at a base URL nothing
    /// listens on (these tests never start a run).
    async fn runtime(pool: &PgPool, secret: &str) -> Arc<crate::runtime::OperatorRuntime> {
        let (name,): (String,) = sqlx::query_as("SELECT current_database()")
            .fetch_one(pool)
            .await
            .unwrap();
        let base = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let (prefix, _) = base.rsplit_once('/').expect("a database in DATABASE_URL");
        Arc::new(crate::runtime::OperatorRuntime::new(
            pool.clone(),
            format!("{prefix}/{name}"),
            "http://127.0.0.1:1",
            Arc::from(secret),
            Arc::new(|_: &mut capsule_corp::sdk::Session<operator::host::Record>| {}),
        ))
    }

    async fn app(pool: PgPool, secret: Option<&str>) -> Router {
        let runtime = runtime(&pool, secret.unwrap_or(SECRET)).await;
        let state = AppState {
            pool,
            jwt_secret: JWT.into(),
            uploads_dir: std::env::temp_dir(),
            cookie_secure: false,
            llm: None,
            auth_limiter: Arc::new(AuthLimiter::new(AuthLimits::default())),
            operator_secret: secret.map(Arc::from),
            operator_model: None,
            runtime,
        };
        crate::routes::router().with_state(state)
    }

    async fn learner(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
            .bind(email)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn workspace(pool: &PgPool, user: Uuid) -> Uuid {
        sqlx::query_scalar("INSERT INTO workspaces (user_id) VALUES ($1) RETURNING id")
            .bind(user)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn jwt(user: Uuid) -> String {
        let now = chrono_now();
        let claims = TokenClaims {
            sub: user.to_string(),
            iat: now,
            exp: now + 3600,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(JWT.as_bytes()),
        )
        .unwrap()
    }

    fn chrono_now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn activity(prompt: &str) -> Value {
        json!({ "kind": "recall", "revision": { "prompt": prompt, "answerKey": "the discounted sum of future rewards" } })
    }

    /// The effect as the capsule applies it: `[path kind prompt answer-key]`,
    /// spelled from the same JSON the manual route takes.
    fn effect_body(id: &str, path: &str, activity: Value) -> Value {
        let payload = json!([
            path,
            activity["kind"],
            activity["revision"]["prompt"],
            activity["revision"]
                .get("answerKey")
                .cloned()
                .unwrap_or(Value::Null)
        ]);
        json!({ "id": id, "capability": PRESENT_FAMILY, "payload": payload })
    }

    fn post(uri: &str, bearer: Option<&str>, body: &Value) -> Request<Body> {
        let mut req = Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = bearer {
            req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(id) = body.get("id").and_then(Value::as_str) {
            req = req.header("Idempotency-Key", id);
        }
        req.body(Body::from(body.to_string())).unwrap()
    }

    fn effect_uri(workspace: Uuid) -> String {
        format!("/internal/effects/{workspace}/{PRESENT_PATH}")
    }

    async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    /// (activities, activity_revisions, effect_receipts) row counts.
    async fn counts(pool: &PgPool) -> (i64, i64, i64) {
        let (a, r, e): (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM activities), (SELECT count(*) FROM activity_revisions),
                    (SELECT count(*) FROM effect_receipts)",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        (a, r, e)
    }

    /// Every column of an activity and its revision that is not an id or a
    /// timestamp, for one activity, as JSON.
    async fn stored(pool: &PgPool, activity_id: Uuid) -> Value {
        let row: (Value,) = sqlx::query_as(
            "SELECT jsonb_build_object(
                 'user_id', a.user_id, 'concept_id', a.concept_id, 'kind', a.kind::text,
                 'revision', r.revision, 'prompt', r.prompt, 'options', r.options,
                 'answer_key', r.answer_key, 'rubric', r.rubric,
                 'source_resource_id', r.source_resource_id, 'source_artifact_id', r.source_artifact_id,
                 'source_location', r.source_location)
             FROM activities a JOIN activity_revisions r ON r.activity_id = a.id
             WHERE a.id = $1",
        )
        .bind(activity_id)
        .fetch_one(pool)
        .await
        .unwrap();
        row.0
    }

    fn without_ids(mut v: Value) -> Value {
        let o = v.as_object_mut().unwrap();
        o.remove("id");
        o.remove("createdAt");
        let c = o.get_mut("current").unwrap().as_object_mut().unwrap();
        c.remove("id");
        c.remove("activityId");
        c.remove("createdAt");
        v
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn the_manual_route_and_the_endpoint_write_the_same_rows(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let prompt = "What does the return G_t sum?";

        let (status, manual) = send(
            &app,
            post("/api/activities", Some(&jwt(user)), &activity(prompt)),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{manual}");

        let body = effect_body(
            "sha256:present-1",
            &format!("workspaces/{ws}/goal/1"),
            activity(prompt),
        );
        let (status, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &body)).await;
        assert_eq!(status, StatusCode::OK, "{reply}");
        let published = reply["value"].clone();

        // Same response shape, same withheld answer key, same stored columns.
        assert_eq!(without_ids(published.clone()), without_ids(manual.clone()));
        assert!(published["current"].get("answerKey").is_none());
        let manual_id: Uuid = manual["id"].as_str().unwrap().parse().unwrap();
        let published_id: Uuid = published["id"].as_str().unwrap().parse().unwrap();
        assert_ne!(manual_id, published_id);
        assert_eq!(
            stored(&pool, manual_id).await,
            stored(&pool, published_id).await
        );

        // One receipt, under the effect id, holding exactly the reply.
        let receipt = receipts::find(&pool, ws, "sha256:present-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(receipt.family, PRESENT_FAMILY);
        assert_eq!(receipt.payload, published);
        assert_eq!(counts(&pool).await, (2, 2, 1));
    }

    fn assess_body(id: &str, path: &str, attempt: &str, outcome: &str, feedback: &str) -> Value {
        json!({ "id": id, "capability": ASSESS_FAMILY, "payload": [path, attempt, outcome, feedback] })
    }

    fn assess_uri(workspace: Uuid) -> String {
        format!("/internal/effects/{workspace}/{ASSESS_PATH}")
    }

    async fn assess_counts(pool: &PgPool) -> (i64, i64) {
        sqlx::query_as(
            "SELECT (SELECT count(*) FROM assessments), (SELECT count(*) FROM effect_receipts WHERE family = 'learning/assess')",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    /// The operator's assessment lands as the attempt's assessment, method
    /// model, with one receipt; a replay writes nothing; an attempt that is
    /// not this learner's is answered, not refused, and writes nothing.
    #[sqlx::test(migrations = "./migrations")]
    async fn the_operator_assesses_an_attempt_once(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let path = format!("workspaces/{ws}/activities");
        let (_, published) = send(
            &app,
            post(
                &effect_uri(ws),
                Some(SECRET),
                &effect_body("sha256:p1", &path, activity("Define the return.")),
            ),
        )
        .await;
        let revision: Uuid = published["value"]["current"]["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let svc = app::services::attempt::AttemptService::new(
            infra::repositories::attempt::PgAttemptRepository::new(pool.clone()),
        );
        let recorded = svc
            .record(
                user,
                app::dtos::attempt::RecordAttemptRequest {
                    request_key: "k1".into(),
                    activity_revision_id: revision,
                    response: json!("the sum of discounted future rewards"),
                    assistance: vec![],
                },
            )
            .await
            .unwrap()
            .receipt;
        assert_eq!(
            recorded.status, "pending",
            "free text waits for the operator"
        );

        let feedback = "Right: and the discount is $\\gamma$.";
        let body = assess_body(
            "sha256:a1",
            &path,
            &recorded.attempt_id,
            "correct",
            feedback,
        );
        let (status, reply) = send(&app, post(&assess_uri(ws), Some(SECRET), &body)).await;
        assert_eq!(status, StatusCode::OK, "{reply}");
        assert_eq!(reply["value"]["assessed"], true, "{reply}");
        let after = svc
            .get(user, recorded.attempt_id.parse().unwrap())
            .await
            .unwrap();
        assert_eq!(after.status, "correct");
        let assessment = after.assessment.unwrap();
        assert_eq!(assessment.method, domain::learning::AssessmentMethod::Model);
        assert_eq!(assessment.feedback, feedback);
        assert_eq!(assess_counts(&pool).await, (1, 1));

        // Replay: the same observation, nothing more written.
        let (_, replay) = send(&app, post(&assess_uri(ws), Some(SECRET), &body)).await;
        assert_eq!(replay, reply);
        assert_eq!(assess_counts(&pool).await, (1, 1));

        // Someone else's attempt, or a word that is not an outcome: answered
        // as not assessed, receipted, and no assessment row.
        let other = learner(&pool, "other@example.com").await;
        let other_ws = workspace(&pool, other).await;
        let foreign = assess_body(
            "sha256:a2",
            &format!("workspaces/{other_ws}/activities"),
            &recorded.attempt_id,
            "correct",
            "x",
        );
        let (_, denied) = send(&app, post(&assess_uri(other_ws), Some(SECRET), &foreign)).await;
        assert_eq!(denied["value"]["assessed"], false, "{denied}");
        let odd = assess_body("sha256:a3", &path, &recorded.attempt_id, "brilliant", "x");
        let (_, odd) = send(&app, post(&assess_uri(ws), Some(SECRET), &odd)).await;
        assert_eq!(odd["value"]["assessed"], false, "{odd}");
        assert_eq!(assess_counts(&pool).await, (1, 3));
    }

    /// A source with page text, owned by `user`.
    async fn source(pool: &PgPool, user: Uuid, title: &str, pages: Vec<String>) -> Uuid {
        let topic: Uuid =
            sqlx::query_scalar("INSERT INTO topics (user_id, name) VALUES ($1, 'T') RETURNING id")
                .bind(user)
                .fetch_one(pool)
                .await
                .unwrap();
        sqlx::query_scalar(
            "INSERT INTO resources (user_id, topic_id, title, resource_type, content_pages)
             VALUES ($1, $2, $3, 'pdf', $4) RETURNING id",
        )
        .bind(user)
        .bind(topic)
        .bind(title)
        .bind(json!(pages))
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn read_body(id: &str, path: &str, resource: &str, from: u64, to: u64) -> Value {
        json!({ "id": id, "capability": SOURCE_FAMILY, "payload": [path, resource, from, to] })
    }

    /// The coach reads pages of the learner's own source: capped in pages
    /// and characters, no receipt; another learner's source is not found.
    #[sqlx::test(migrations = "./migrations")]
    async fn the_coach_reads_pages_of_the_learners_source(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let path = format!("workspaces/{ws}/activities");
        let long = "x".repeat(READ_MAX_CHARS + 50);
        let pages: Vec<String> = (1..=6)
            .map(|n| {
                if n == 2 {
                    long.clone()
                } else {
                    format!("page {n} text")
                }
            })
            .collect();
        let book = source(&pool, user, "RL book", pages).await;
        let uri = format!("/internal/effects/{ws}/{SOURCE_PATH}");

        let (status, reply) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &read_body("sha256:r1", &path, &book.to_string(), 1, 10),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{reply}");
        let got = &reply["value"];
        assert_eq!(got["found"], true, "{got}");
        assert_eq!(got["title"], "RL book");
        assert_eq!(
            (&got["pageCount"], &got["from"], &got["to"]),
            (&json!(6), &json!(1), &json!(4))
        );
        let texts: Vec<&str> = got["pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["text"].as_str().unwrap())
            .collect();
        assert_eq!(texts[0], "page 1 text");
        assert_eq!(
            texts[1].chars().count(),
            READ_MAX_CHARS + 1,
            "cut, with a mark"
        );
        assert!(texts[1].ends_with('…'));
        assert_eq!(got["pages"][3]["page"], 4);

        // Past the end clamps; a read of one page is one page.
        let (_, tail) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &read_body("sha256:r2", &path, &book.to_string(), 6, 9),
            ),
        )
        .await;
        assert_eq!(
            (&tail["value"]["from"], &tail["value"]["to"]),
            (&json!(6), &json!(6))
        );

        // Another learner's source, or no such id: not found, not refused.
        let other = learner(&pool, "other@example.com").await;
        let theirs = source(&pool, other, "Theirs", vec!["secret".into()]).await;
        let (_, denied) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &read_body("sha256:r3", &path, &theirs.to_string(), 1, 1),
            ),
        )
        .await;
        assert_eq!(denied["value"]["found"], false, "{denied}");
        assert!(denied.to_string().contains("secret") == false);
        let (_, odd) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &read_body("sha256:r4", &path, "not-an-id", 1, 1),
            ),
        )
        .await;
        assert_eq!(odd["value"]["found"], false, "{odd}");

        // Reads leave no receipt.
        let (receipts,): (i64,) = sqlx::query_as("SELECT count(*) FROM effect_receipts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(receipts, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_replayed_effect_id_answers_the_receipt_and_writes_nothing(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let path = format!("workspaces/{ws}/goal/1");

        let first = effect_body("sha256:present-1", &path, activity("First prompt"));
        let (_, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &first)).await;
        let before = counts(&pool).await;
        assert_eq!(before, (1, 1, 1));

        // Same id, even with different operands: the recorded observation.
        let again = effect_body("sha256:present-1", &path, activity("A different prompt"));
        let (status, replay) = send(&app, post(&effect_uri(ws), Some(SECRET), &again)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(replay, reply);
        assert_eq!(counts(&pool).await, before);

        // A new id is a new effect.
        let next = effect_body("sha256:present-2", &path, activity("A different prompt"));
        let (_, second) = send(&app, post(&effect_uri(ws), Some(SECRET), &next)).await;
        assert_ne!(second["value"]["id"], reply["value"]["id"]);
        assert_eq!(counts(&pool).await, (2, 2, 2));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unauthorized_misbound_or_unknown_requests_write_nothing(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let other = workspace(&pool, learner(&pool, "other@example.com").await).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let body = effect_body(
            "sha256:present-1",
            &format!("workspaces/{ws}/goal/1"),
            activity("Prompt"),
        );

        // No secret, a wrong secret, a learner's own token: all 401.
        for bearer in [None, Some("not-the-secret"), Some(jwt(user).as_str())] {
            let (status, _) = send(&app, post(&effect_uri(ws), bearer, &body)).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }

        // A path outside the URL's workspace is refused in a 200.
        let misbound = effect_body(
            "sha256:present-1",
            &format!("workspaces/{other}/goal/1"),
            activity("Prompt"),
        );
        let (status, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &misbound)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            reply["refused"].as_str().unwrap().contains("outside"),
            "{reply}"
        );

        // The capability must be the family this URL serves.
        let mut wrong = body.clone();
        wrong["capability"] = json!("learner/answer");
        let (_, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &wrong)).await;
        assert!(reply.get("refused").is_some(), "{reply}");

        // An unknown family or workspace is 404; a missing secret unmounts all.
        let (status, _) = send(
            &app,
            post(
                &format!("/internal/effects/{ws}/learner.attempt"),
                Some(SECRET),
                &body,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = send(&app, post(&effect_uri(Uuid::new_v4()), Some(SECRET), &body)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let unmounted = self::app(pool.clone(), None).await;
        let (status, _) = send(&unmounted, post(&effect_uri(ws), Some(SECRET), &body)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        assert_eq!(counts(&pool).await, (0, 0, 0));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_refused_activity_leaves_no_receipt(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET)).await;
        let path = format!("workspaces/{ws}/goal/1");

        // Invalid content, and a kind the domain has no name for: both
        // refused through the service's own checks, nothing written, no
        // receipt, so a corrected program is free to perform under this id.
        let blank = effect_body("sha256:present-1", &path, activity("   "));
        let (status, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &blank)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(reply["refused"], "prompt is required");

        let unknown_kind = effect_body(
            "sha256:present-1",
            &path,
            json!({ "kind": "lecture", "revision": { "prompt": "P", "answerKey": "A" } }),
        );
        let (_, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &unknown_kind)).await;
        assert!(
            reply["refused"].as_str().unwrap().contains("kind"),
            "{reply}"
        );

        // The path as the kernel hands it on, a segment list, is the same
        // path; a list naming another workspace is outside the scope.
        let listed = json!({ "id": "sha256:present-1", "capability": PRESENT_FAMILY,
            "payload": [["workspaces", Uuid::new_v4().to_string(), "activities"], "recall", "P", "A"] });
        let (_, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &listed)).await;
        assert!(
            reply["refused"].as_str().unwrap().contains("outside"),
            "{reply}"
        );

        assert_eq!(counts(&pool).await, (0, 0, 0));
        let segments = json!({ "id": "sha256:present-1", "capability": PRESENT_FAMILY,
            "payload": [["workspaces", ws.to_string(), "activities"], "recall", "P", "A"] });
        let (status, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &segments)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(reply["value"]["current"]["prompt"], "P", "{reply}");
        assert_eq!(counts(&pool).await, (1, 1, 1));
    }
}
