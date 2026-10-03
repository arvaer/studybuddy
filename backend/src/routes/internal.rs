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
//! Two families. `learning/present`, at `learning.present`: the reply and
//! the receipt are the same JSON the manual route answers, produced by the
//! same `ActivityService::create` on one transaction with the receipt: both
//! land or neither does, and a replayed id answers the receipt and writes
//! nothing. `call/model`, at `call.model` (19b): the server's `LlmClient`
//! over the `agent.v2` layout in `model_provider`; the reply is receipted
//! once it exists, so a replayed id answers the same form without asking
//! the model again. A failure before any text came back is `declined`
//! (unreachable, an error status) or `refused` (the request itself); one
//! after the call may have run is `unknown` (timeout, unreadable answer),
//! which parks the effect for the host to settle (H3). `learner/attempt`
//! arrives with its caller in 20b.

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

use crate::llm::LlmError;
use crate::model_provider;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/internal/effects/{workspace}/{family}", post(effect))
}

/// The families served, each as the capsule applies it (what the request's
/// `capability` must say) and as the URL spells it, where a slash cannot go.
const PRESENT_PATH: &str = "learning.present";
const PRESENT_FAMILY: &str = "learning/present";
const MODEL_PATH: &str = "call.model";
const MODEL_FAMILY: &str = "call/model";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Present,
    Model,
}

impl Family {
    fn at(path: &str) -> Option<Self> {
        match path {
            PRESENT_PATH => Some(Self::Present),
            MODEL_PATH => Some(Self::Model),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Present => PRESENT_FAMILY,
            Self::Model => MODEL_FAMILY,
        }
    }
}

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

/// This call will not be performed and nothing was; the run may go on with
/// `("declined" why)` as the call's value (V1-06d). No receipt.
fn declined(why: impl Into<String>) -> Response {
    Json(json!({ "declined": why.into() })).into_response()
}

/// The call may have run and its result is lost to us: the effect parks
/// uncertain and the host settles it (H3). No receipt, since there is no
/// observation to complete with.
fn unknown(why: impl Into<String>) -> Response {
    Json(json!({ "unknown": why.into() })).into_response()
}

/// The model client's failures in the protocol's words. Validation is the
/// request's own fault; an unreachable provider or an error status sent
/// nothing the model acted on; a timeout or an unreadable answer came
/// after it may have.
fn model_failure(e: LlmError) -> Response {
    match e {
        LlmError::NotConfigured => refused("no model provider is configured on this server"),
        LlmError::Validation(why) => refused(why),
        LlmError::Transport => declined("the model provider could not be reached"),
        LlmError::Provider(status) => {
            declined(format!("the model provider answered status {status}"))
        }
        LlmError::Timeout => unknown("the model provider timed out; the call may have run"),
        LlmError::Malformed => {
            unknown("the model provider answered unreadably; the call may have run")
        }
    }
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
    let Some(family) = Family::at(&family) else {
        return StatusCode::NOT_FOUND.into_response();
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
    if req.capability != family.name() {
        return refused(format!(
            "this endpoint serves {}, not {}",
            family.name(),
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

    let served = match family {
        Family::Present => present(&state, workspace, owner, &req).await,
        Family::Model => model(&state, workspace, &req).await,
    };
    match served {
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

/// `call/model`: the receipt if the call already ran, else the server's
/// model over the `agent.v2` layout, receipted once its answer exists. The
/// receipt is its own short transaction: there is no domain change to share
/// one with, and the call itself cannot be inside one.
async fn model(
    state: &AppState,
    workspace: Uuid,
    req: &EffectRequest,
) -> Result<Response, Response> {
    if let Some(receipt) = receipts::find(&state.pool, workspace, &req.id)
        .await
        .map_err(database)?
    {
        return Ok(value(receipt.payload));
    }

    let [request] = req.payload.as_slice() else {
        return Err(refused(format!(
            "{MODEL_FAMILY} takes one {} request",
            model_provider::KIND
        )));
    };
    let messages = model_provider::messages(request).map_err(refused)?;
    let client = state
        .llm
        .as_deref()
        .ok_or_else(|| model_failure(LlmError::NotConfigured))?;
    let text = client
        .complete(&messages, None)
        .await
        .map_err(model_failure)?;

    let receipt = Receipt {
        effect_id: req.id.clone(),
        workspace_id: workspace,
        family: MODEL_FAMILY.to_string(),
        payload: model_provider::reply(&text),
    };
    let mut tx = state.pool.begin().await.map_err(database)?;
    match receipts::record(&mut tx, &receipt)
        .await
        .map_err(database)?
    {
        Recorded::New => {
            tx.commit().await.map_err(database)?;
            Ok(value(receipt.payload))
        }
        // Two calls under one id raced to the provider; the first answer
        // recorded is the one the run observes, now and on replay.
        Recorded::Existing(earlier) => Ok(value(earlier.payload)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LlmClient, LlmProvider, LlmSettings};
    use app::dtos::auth::TokenClaims;
    use app::services::rate_limit::{AuthLimiter, AuthLimits};
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tower::ServiceExt;

    const SECRET: &str = "operator-secret-for-tests";
    const JWT: &str = "jwt-secret-for-tests";

    fn app(pool: PgPool, secret: Option<&str>) -> Router {
        app_with_model(pool, secret, None)
    }

    fn app_with_model(pool: PgPool, secret: Option<&str>, llm: Option<LlmClient>) -> Router {
        let state = AppState {
            pool,
            jwt_secret: JWT.into(),
            uploads_dir: std::env::temp_dir(),
            cookie_secure: false,
            llm: llm.map(Arc::new),
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

    // ---- call/model ----

    /// A chat-completions provider that answers `form`, or fails as `mode`
    /// says, and keeps every request body it saw.
    struct Fake {
        mode: &'static str,
        form: &'static str,
        seen: Mutex<Vec<Value>>,
    }

    async fn completions(
        State(fake): State<Arc<Fake>>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Response {
        assert!(
            headers.get("authorization").is_some(),
            "the server's key was not sent"
        );
        fake.seen.lock().unwrap().push(body);
        match fake.mode {
            "ok" => Json(json!({ "choices": [{ "message": { "role": "assistant", "content": fake.form } }] }))
                .into_response(),
            "busy" => (StatusCode::TOO_MANY_REQUESTS, "slow down").into_response(),
            "slow" => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                StatusCode::OK.into_response()
            }
            other => unreachable!("{other}"),
        }
    }

    async fn provider(mode: &'static str, form: &'static str) -> (LlmClient, Arc<Fake>) {
        let fake = Arc::new(Fake {
            mode,
            form,
            seen: Mutex::new(Vec::new()),
        });
        let app = Router::new()
            .route("/v1/chat/completions", axum::routing::post(completions))
            .with_state(Arc::clone(&fake));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = LlmClient::new(LlmSettings {
            provider: LlmProvider::OpenAi,
            model: "test-model".into(),
            api_key: "server-side-key".into(),
            base_url: reqwest::Url::parse(&format!("http://{addr}/")).unwrap(),
            timeout: Duration::from_millis(300),
        })
        .unwrap();
        (client, fake)
    }

    fn agent_request(task: &str) -> Value {
        json!([
            model_provider::KIND,
            "Coach the learner.",
            [],
            task,
            [[{ "form": "(coach/ask \"workspaces/w/a\" \"q?\")", "content": "…" }, [["answer", "workspaces/w/a", "42"]]]],
            [["coach/ask", "Present an activity and wait.", ["path", "prompt"]], ["coach/finish", "Finish.", ["summary"]]]
        ])
    }

    fn model_body(id: &str, request: Value) -> Value {
        json!({ "id": id, "capability": MODEL_FAMILY, "payload": [request] })
    }

    fn model_uri(workspace: Uuid) -> String {
        format!("/internal/effects/{workspace}/{MODEL_PATH}")
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_model_call_is_laid_out_answered_as_a_form_and_receipted(pool: PgPool) {
        let ws = workspace(&pool, learner(&pool, "m@x.test").await).await;
        let (client, fake) = provider("ok", "(coach/finish \"the learner has it\")").await;
        let app = app_with_model(pool.clone(), Some(SECRET), Some(client));

        let (status, reply) = send(
            &app,
            post(
                &model_uri(ws),
                Some(SECRET),
                &model_body("sha256:m1", agent_request("Teach discounting")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let expected = json!({
            "value": { "form": "(coach/finish \"the learner has it\")", "content": "(coach/finish \"the learner has it\")" }
        });
        assert_eq!(reply, expected);

        // The layout the provider saw: system with the verbs, the task, then
        // the turn as what we said and what came of it.
        let seen = fake.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        let messages = seen[0]["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "system");
        let system = messages[0]["content"].as_str().unwrap();
        assert!(system.starts_with("Coach the learner."), "{system}");
        assert!(
            system.contains("(coach/ask path prompt) — Present an activity and wait."),
            "{system}"
        );
        assert!(!system.contains("server-side-key"));
        assert_eq!(
            messages[1],
            json!({ "role": "user", "content": "Teach discounting" })
        );
        assert_eq!(
            messages[2],
            json!({ "role": "assistant", "content": "(coach/ask \"workspaces/w/a\" \"q?\")" })
        );
        assert_eq!(
            messages[3],
            json!({ "role": "user", "content": "[\"answer\",\"workspaces/w/a\",\"42\"]" })
        );

        // Receipted under the family, with exactly the reply.
        let (family, payload): (String, Value) = sqlx::query_as(
            "SELECT family, payload FROM effect_receipts WHERE effect_id = 'sha256:m1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(family, MODEL_FAMILY);
        assert_eq!(payload, expected["value"]);

        // A replay answers the receipt and asks the model nothing, whatever
        // the operands now say; a new id is a new call.
        let (status, again) = send(
            &app,
            post(
                &model_uri(ws),
                Some(SECRET),
                &model_body("sha256:m1", agent_request("Something else")),
            ),
        )
        .await;
        assert_eq!((status, again), (StatusCode::OK, expected));
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
        let (status, _) = send(
            &app,
            post(
                &model_uri(ws),
                Some(SECRET),
                &model_body("sha256:m2", agent_request("Something else")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fake.seen.lock().unwrap().len(), 2);
        assert_eq!(counts(&pool).await, (0, 0, 2));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn model_failures_speak_the_protocol_and_leave_no_receipt(pool: PgPool) {
        let ws = workspace(&pool, learner(&pool, "f@x.test").await).await;
        let uri = model_uri(ws);

        // An error status: nothing the model acted on, so declined.
        let (client, fake) = provider("busy", "").await;
        let app = app_with_model(pool.clone(), Some(SECRET), Some(client));
        let (status, reply) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &model_body("sha256:f1", agent_request("t")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(reply["declined"], "the model provider answered status 429");
        assert_eq!(fake.seen.lock().unwrap().len(), 1);

        // A timeout: the call may have run, so unknown; the host settles it.
        let (client, _) = provider("slow", "").await;
        let app = app_with_model(pool.clone(), Some(SECRET), Some(client));
        let (status, reply) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &model_body("sha256:f2", agent_request("t")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            reply["unknown"].as_str().unwrap().contains("timed out"),
            "{reply}"
        );

        // A model that wrote prose: a value with no form and the reason,
        // which act hands the capsule as a refusal it can tell the model.
        let (client, _) = provider("ok", "I would ask about discounting next.").await;
        let app = app_with_model(pool.clone(), Some(SECRET), Some(client));
        let (status, reply) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &model_body("sha256:f3", agent_request("t")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(reply["value"].get("form").is_none(), "{reply}");
        assert!(
            reply["value"]["why"].as_str().unwrap().contains("no form"),
            "{reply}"
        );

        // The request's own faults, and a server with no model: refused,
        // before anything is sent.
        let (client, fake) = provider("ok", "(coach/finish \"x\")").await;
        let app = app_with_model(pool.clone(), Some(SECRET), Some(client));
        let mut wrong_kind = agent_request("t");
        wrong_kind[0] = json!("agent.v1");
        let (_, reply) = send(
            &app,
            post(&uri, Some(SECRET), &model_body("sha256:f4", wrong_kind)),
        )
        .await;
        assert!(
            reply["refused"]
                .as_str()
                .unwrap()
                .contains("serves agent.v2"),
            "{reply}"
        );
        let two_operands = json!({ "id": "sha256:f5", "capability": MODEL_FAMILY, "payload": [agent_request("t"), "extra"] });
        let (_, reply) = send(&app, post(&uri, Some(SECRET), &two_operands)).await;
        assert!(
            reply["refused"]
                .as_str()
                .unwrap()
                .contains("takes one agent.v2 request"),
            "{reply}"
        );
        let misnamed = json!({ "id": "sha256:f6", "capability": PRESENT_FAMILY, "payload": [agent_request("t")] });
        let (_, reply) = send(&app, post(&uri, Some(SECRET), &misnamed)).await;
        assert!(
            reply["refused"]
                .as_str()
                .unwrap()
                .contains("serves call/model"),
            "{reply}"
        );
        assert_eq!(fake.seen.lock().unwrap().len(), 0);

        let app = app_with_model(pool.clone(), Some(SECRET), None);
        let (status, reply) = send(
            &app,
            post(
                &uri,
                Some(SECRET),
                &model_body("sha256:f7", agent_request("t")),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            reply["refused"],
            "no model provider is configured on this server"
        );

        // The prose reply was a value, so it was receipted; nothing else was.
        assert_eq!(counts(&pool).await, (0, 0, 1));
    }
}
