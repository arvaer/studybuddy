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
//! One family today, `learning/present`, at `learning.present`. The reply
//! and the receipt are the same JSON the manual route answers, produced by
//! the same `ActivityService::create` on one transaction with the receipt:
//! both land or neither does, and a replayed id answers the receipt and
//! writes nothing. `learner/attempt` arrives with its caller in 20b.

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

use app::dtos::activity::CreateActivityRequest;
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
    if family != PRESENT_PATH {
        return StatusCode::NOT_FOUND.into_response();
    }
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
    if req.capability != PRESENT_FAMILY {
        return refused(format!(
            "{PRESENT_PATH} serves {PRESENT_FAMILY}, not {}",
            req.capability
        ));
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

    match present(&state, workspace, owner, &req).await {
        Ok(reply) | Err(reply) => reply,
    }
}

/// `[path, activity]`: the scope path the environment checked, then the
/// activity exactly as the manual route receives it (`CreateActivityRequest`).
fn parse_present(workspace: Uuid, payload: &[Value]) -> Result<CreateActivityRequest, Response> {
    let [path, activity] = payload else {
        return Err(refused(format!("{PRESENT_FAMILY} takes [path, activity]")));
    };
    let Some(path) = path.as_str() else {
        return Err(refused("path must be a string"));
    };
    let scope = format!("workspaces/{workspace}/");
    if !path.starts_with(&scope) {
        return Err(refused(format!("path is outside {scope}*")));
    }
    serde_json::from_value(activity.clone()).map_err(|e| refused(format!("activity: {e}")))
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

    fn app(pool: PgPool, secret: Option<&str>) -> Router {
        let state = AppState {
            pool,
            jwt_secret: JWT.into(),
            uploads_dir: std::env::temp_dir(),
            cookie_secure: false,
            llm: None,
            auth_limiter: Arc::new(AuthLimiter::new(AuthLimits::default())),
            operator_secret: secret.map(Arc::from),
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

    fn effect_body(id: &str, path: &str, activity: Value) -> Value {
        json!({ "id": id, "capability": PRESENT_FAMILY, "payload": [path, activity] })
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
        let app = app(pool.clone(), Some(SECRET));
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

    #[sqlx::test(migrations = "./migrations")]
    async fn a_replayed_effect_id_answers_the_receipt_and_writes_nothing(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET));
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
        let app = app(pool.clone(), Some(SECRET));
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
        let unmounted = self::app(pool.clone(), None);
        let (status, _) = send(&unmounted, post(&effect_uri(ws), Some(SECRET), &body)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        assert_eq!(counts(&pool).await, (0, 0, 0));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_refused_activity_leaves_no_receipt(pool: PgPool) {
        let user = learner(&pool, "owner@example.com").await;
        let ws = workspace(&pool, user).await;
        let app = app(pool.clone(), Some(SECRET));
        let path = format!("workspaces/{ws}/goal/1");

        // Invalid content, and a concept another learner owns: both refused
        // through the service's own checks, nothing written, no receipt, so
        // a corrected program is free to perform under this id.
        let blank = effect_body("sha256:present-1", &path, activity("   "));
        let (status, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &blank)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(reply["refused"], "prompt is required");

        let foreign = effect_body(
            "sha256:present-1",
            &path,
            json!({ "kind": "recall", "conceptId": Uuid::new_v4(), "revision": { "prompt": "P", "answerKey": "A" } }),
        );
        let (_, reply) = send(&app, post(&effect_uri(ws), Some(SECRET), &foreign)).await;
        assert!(
            reply["refused"].as_str().unwrap().contains("concept"),
            "{reply}"
        );

        assert_eq!(counts(&pool).await, (0, 0, 0));
        let (status, _) = send(
            &app,
            post(
                &effect_uri(ws),
                Some(SECRET),
                &effect_body("sha256:present-1", &path, activity("P")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(counts(&pool).await, (1, 1, 1));
    }
}
