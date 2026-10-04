//! The learner's workspace and goal (20a). One text box: the intent goes in,
//! the operator starts, and the page reads where the operator is.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use infra::repositories::workspace::{Goal, PgWorkspaceRepository};

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::runtime::OperatorState;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces/current", get(current))
        .route("/workspaces/{id}", get(get_one))
        .route("/workspaces/{id}/goal", post(set_goal))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceResponse {
    pub id: Uuid,
    pub goal: Option<Goal>,
    #[serde(flatten)]
    pub state: OperatorState,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalRequest {
    pub intent: String,
}

fn repo(state: &AppState) -> PgWorkspaceRepository {
    PgWorkspaceRepository::new(state.pool.clone())
}

async fn describe(state: &AppState, workspace: Uuid) -> Result<WorkspaceResponse, HttpError> {
    let goal = repo(state).goal(workspace).await?;
    let operator = match goal {
        // No goal, no session: nothing to open.
        None => OperatorState::Idle { last: None },
        Some(_) => state
            .runtime
            .state(workspace)
            .await
            .map_err(operator_error)?,
    };
    Ok(WorkspaceResponse {
        id: workspace,
        goal,
        state: operator,
    })
}

fn operator_error(e: operator::OperatorError) -> HttpError {
    tracing::error!("operator: {e}");
    HttpError(app::errors::AppError::Domain(
        domain::errors::DomainError::Repository("the operator is unavailable".into()),
    ))
}

/// The learner's workspace, made on first sight.
async fn current(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    let workspace = repo(&state).find_or_create(user_id).await?;
    Ok(Json(describe(&state, workspace).await?))
}

async fn get_one(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, HttpError> {
    let workspace = repo(&state).owned(user_id, id).await?;
    Ok(Json(describe(&state, workspace).await?))
}

/// The intent in: stored as goal revision 1, and the run that serves it
/// starts now, in the background. Answers 202 with the workspace thinking.
async fn set_goal(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<GoalRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let workspace = repo(&state).owned(user_id, id).await?;
    let goal = repo(&state).set_goal(workspace, &req.intent).await?;
    state.runtime.start(workspace, goal.id, goal.intent.clone());
    Ok((
        StatusCode::ACCEPTED,
        Json(describe(&state, workspace).await?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Installer, OperatorRuntime};
    use app::dtos::auth::TokenClaims;
    use app::services::rate_limit::{AuthLimiter, AuthLimits};
    use axum::body::Body;
    use axum::http::{header, Request};
    use capsule_corp::sdk::{Effect, Reply, Session};
    use http_body_util::BodyExt;
    use operator::host::Record;
    use serde_json::{json, Value};
    use sqlx::PgPool;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tower::ServiceExt;

    const JWT: &str = "jwt-secret-for-tests";
    const SECRET: &str = "operator-secret-for-tests";
    const FIRST: &str =
        r#"(coach/present "What does the return G_t sum?" "the discounted sum of future rewards")"#;
    const FOLLOW_UP: &str =
        r#"(coach/present "Why discount at all?" "to bound the sum and prefer sooner reward")"#;
    const FINISH: &str =
        r#"(coach/finish "You can define the return and say why it is discounted.")"#;

    type Calls = Arc<Mutex<Vec<Effect>>>;

    /// A scripted model answering `forms` in order and keeping each request.
    fn scripted(models: Calls, forms: Vec<&'static str>) -> Installer {
        scripted_dropping(models, forms, Arc::new(AtomicUsize::new(0)))
    }

    /// The same, but while `drops` is above zero each call answers
    /// `unknown` (the connection dropped) and takes one off: the adapter
    /// after its retries, or the process dying mid-call.
    fn scripted_dropping(
        models: Calls,
        forms: Vec<&'static str>,
        drops: Arc<AtomicUsize>,
    ) -> Installer {
        let forms = Mutex::new(forms.into_iter());
        Arc::new(move |session: &mut Session<Record>| {
            let models = Arc::clone(&models);
            let drops = Arc::clone(&drops);
            let mut forms: Vec<&'static str> = forms.lock().unwrap().clone().collect();
            session.provide("call/model", move |effect: &Effect| {
                models.lock().unwrap().push(effect.clone());
                if drops
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                    .is_ok()
                {
                    return Reply::Unknown("the connection dropped".into());
                }
                if forms.is_empty() {
                    return Reply::Declined("the script is over".into());
                }
                Reply::Value(json!({ "form": forms.remove(0) }))
            });
        })
    }

    async fn test_database_url(pool: &PgPool) -> String {
        let (name,): (String,) = sqlx::query_as("SELECT current_database()")
            .fetch_one(pool)
            .await
            .unwrap();
        let base = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let (prefix, _) = base.rsplit_once('/').expect("a database in DATABASE_URL");
        format!("{prefix}/{name}")
    }

    /// The app served on loopback, so the owner's `learning/present` client
    /// reaches this process's own endpoint; the same router answers the
    /// test's requests through `oneshot`.
    async fn serve(pool: &PgPool, installer: Installer) -> (Router, Arc<OperatorRuntime>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let runtime = Arc::new(OperatorRuntime::new(
            pool.clone(),
            test_database_url(pool).await,
            base_url,
            Arc::from(SECRET),
            installer,
        ));
        let state = AppState {
            pool: pool.clone(),
            jwt_secret: JWT.into(),
            uploads_dir: std::env::temp_dir(),
            cookie_secure: false,
            llm: None,
            auth_limiter: Arc::new(AuthLimiter::new(AuthLimits::default())),
            operator_secret: Some(Arc::from(SECRET)),
            operator_model: None,
            runtime: Arc::clone(&runtime),
        };
        let app = crate::routes::router().with_state(state);
        let served = app.clone();
        tokio::spawn(async move { axum::serve(listener, served).await.unwrap() });
        (app, runtime)
    }

    async fn learner(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar("INSERT INTO users (email, password_hash, display_name) VALUES ($1, 'x', 'L') RETURNING id")
            .bind(email)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn jwt(user: Uuid) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
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

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        user: Uuid,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", jwt(user)))
            .header(header::CONTENT_TYPE, "application/json");
        let body = match body {
            Some(body) => Body::from(body.to_string()),
            None => Body::empty(),
        };
        let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    /// The workspace once the operator is no longer thinking.
    async fn settled(app: &Router, user: Uuid, ws: &str) -> Value {
        for _ in 0..100 {
            let (status, body) =
                call(app, "GET", &format!("/api/workspaces/{ws}"), user, None).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            if body["operator"] != "thinking" {
                return body;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the operator thought for over ten seconds");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn an_intent_starts_the_operator_and_the_first_activity_lands(pool: PgPool) {
        let models = Calls::default();
        let (app, runtime) = serve(&pool, scripted(models.clone(), vec![FIRST])).await;
        let user = learner(&pool, "a@x.test").await;
        let other = learner(&pool, "b@x.test").await;

        // The learner's workspace, made on first sight, with no goal.
        let (status, current) = call(&app, "GET", "/api/workspaces/current", user, None).await;
        assert_eq!(status, StatusCode::OK, "{current}");
        assert_eq!(current["goal"], Value::Null);
        assert_eq!(current["operator"], "idle");
        let ws = current["id"].as_str().unwrap().to_string();
        let (_, again) = call(&app, "GET", "/api/workspaces/current", user, None).await;
        assert_eq!(again["id"], current["id"], "one workspace per learner");

        // Another learner sees nothing of it.
        let (status, _) = call(&app, "GET", &format!("/api/workspaces/{ws}"), other, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            other,
            Some(json!({ "intent": "mine" })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // A blank intent is refused before anything starts.
        let (status, _) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": "   " })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        // The intent in: goal revision 1, the operator thinking.
        let intent = "Understand the return and discounting in RL.";
        let (status, accepted) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": intent })),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED, "{accepted}");
        assert_eq!(accepted["goal"]["revision"], 1);
        assert_eq!(accepted["goal"]["intent"], intent);
        assert!(
            accepted["operator"] == "thinking" || accepted["operator"] == "waiting",
            "{accepted}"
        );

        // The first activity lands through this process's own endpoint, and
        // the operator waits on it.
        let waiting = settled(&app, user, &ws).await;
        assert_eq!(waiting["operator"], "waiting", "{waiting}");
        let activity_id = waiting["currentActivityId"].as_str().unwrap();
        let (prompt, count): (String, i64) = sqlx::query_as(
            "SELECT r.prompt, (SELECT count(*) FROM activities)
             FROM activities a JOIN activity_revisions r ON r.activity_id = a.id WHERE a.id = $1::uuid",
        )
        .bind(activity_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(prompt, "What does the return G_t sum?");
        assert_eq!(count, 1);
        let (receipts,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM effect_receipts WHERE family = 'learning/present'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(receipts, 1);
        assert_eq!(
            models.lock().unwrap()[0].payload()[0][3],
            intent,
            "the goal is the model's task"
        );

        // One goal per workspace in Phase 2.
        let (status, _) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": "another" })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);

        // A new process: a fresh runtime reopens the record and reads the
        // same wait, asking no model.
        runtime.close(ws.parse().unwrap()).await.unwrap();
        let (app, runtime) = serve(&pool, scripted(Calls::default(), vec![])).await;
        let (status, reopened) =
            call(&app, "GET", &format!("/api/workspaces/{ws}"), user, None).await;
        assert_eq!(status, StatusCode::OK, "{reopened}");
        assert_eq!(reopened["operator"], "waiting");
        assert_eq!(reopened["currentActivityId"], activity_id);
        assert_eq!(reopened["goal"]["intent"], intent);
        assert_eq!(models.lock().unwrap().len(), 1, "reopen asked no provider");
        runtime.close(ws.parse().unwrap()).await.unwrap();
    }

    async fn revision_of(pool: &PgPool, activity: &str) -> Uuid {
        sqlx::query_scalar("SELECT id FROM activity_revisions WHERE activity_id = $1::uuid ORDER BY revision DESC LIMIT 1")
            .bind(activity)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn an_answer_wakes_the_operator_and_the_follow_up_lands(pool: PgPool) {
        let models = Calls::default();
        let (app, runtime) = serve(
            &pool,
            scripted(models.clone(), vec![FIRST, FOLLOW_UP, FINISH]),
        )
        .await;
        let user = learner(&pool, "c@x.test").await;
        let (_, current) = call(&app, "GET", "/api/workspaces/current", user, None).await;
        let ws = current["id"].as_str().unwrap().to_string();
        let (status, _) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": "Understand the return and discounting in RL." })),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let first = settled(&app, user, &ws).await;
        let first_activity = first["currentActivityId"].as_str().unwrap().to_string();
        let first_revision = revision_of(&pool, &first_activity).await;

        // The learner answers through the ordinary attempts route. The
        // attempt is accepted as always, and the operator wakes: the wait is
        // receipted with the attempt, the model reads it, the follow-up
        // lands, and the workspace waits on it.
        let answer = json!({
            "requestKey": "attempt-1",
            "activityRevisionId": first_revision,
            "response": "the discounted sum of future rewards"
        });
        let (status, receipt) = call(&app, "POST", "/api/attempts", user, Some(answer)).await;
        assert_eq!(status, StatusCode::CREATED, "{receipt}");
        assert_eq!(receipt["status"], "correct");
        let second = settled(&app, user, &ws).await;
        assert_eq!(second["operator"], "waiting", "{second}");
        let second_activity = second["currentActivityId"].as_str().unwrap().to_string();
        assert_ne!(second_activity, first_activity);
        let (prompt,): (String,) = sqlx::query_as(
            "SELECT r.prompt FROM activity_revisions r WHERE r.activity_id = $1::uuid",
        )
        .bind(&second_activity)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(prompt, "Why discount at all?");
        let (wait_receipts, attempt_in_receipt): (i64, bool) = sqlx::query_as(
            "SELECT count(*), bool_and(payload->>'attemptId' = $1)
             FROM effect_receipts WHERE family = 'learner/wait'",
        )
        .bind(receipt["attemptId"].as_str().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((wait_receipts, attempt_in_receipt), (1, true));
        {
            let models = models.lock().unwrap();
            assert_eq!(models.len(), 2);
            let turns = &models[1].payload()[0][4];
            assert_eq!(
                turns[0][1][0][0], "attempt",
                "the model read the attempt as the verb's answer"
            );
            assert_eq!(
                turns[0][1][0][1]["response"],
                "the discounted sum of future rewards"
            );
        }

        // An attempt recorded with no wake (the process died in the gap, or
        // the attempt came another way): the next page read finds it, the
        // wait is receipted, and the run goes on to its end.
        let second_revision = revision_of(&pool, &second_activity).await;
        app::services::attempt::AttemptService::new(
            infra::repositories::attempt::PgAttemptRepository::new(pool.clone()),
        )
        .record(
            user,
            app::dtos::attempt::RecordAttemptRequest {
                request_key: "attempt-2".into(),
                activity_revision_id: second_revision,
                response: json!("sooner reward is worth more"),
                assistance: vec![],
            },
        )
        .await
        .unwrap();
        let finished = settled(&app, user, &ws).await;
        assert_eq!(finished["operator"], "idle", "{finished}");
        assert!(
            finished["last"]
                .as_str()
                .unwrap()
                .contains("You can define the return"),
            "{finished}"
        );
        assert_eq!(models.lock().unwrap().len(), 3);
        let (wait_receipts,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM effect_receipts WHERE family = 'learner/wait'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(wait_receipts, 2);

        // Reading again settles nothing new and asks no model.
        let again = settled(&app, user, &ws).await;
        assert_eq!(again["operator"], "idle");
        assert_eq!(models.lock().unwrap().len(), 3);
        runtime.close(ws.parse().unwrap()).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn an_interrupted_think_is_resumed_once_per_process(pool: PgPool) {
        // The first call to the model is lost: the adapter's retries are
        // spent, or the process died mid-call. The run parks on
        // `call/model` with nothing to complete it from.
        let models = Calls::default();
        let drops = Arc::new(AtomicUsize::new(1));
        let (app, runtime) = serve(
            &pool,
            scripted_dropping(models.clone(), vec![FIRST, FOLLOW_UP], Arc::clone(&drops)),
        )
        .await;
        let user = learner(&pool, "d@x.test").await;
        let (_, current) = call(&app, "GET", "/api/workspaces/current", user, None).await;
        let ws = current["id"].as_str().unwrap().to_string();
        let (status, _) = call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": "Understand the return and discounting in RL." })),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);

        // The next page read allows the park again and the think goes on:
        // the model is asked the same thing a second time, and the first
        // activity lands.
        let first = settled(&app, user, &ws).await;
        assert_eq!(first["operator"], "waiting", "{first}");
        let first_activity = first["currentActivityId"].as_str().unwrap().to_string();
        {
            let models = models.lock().unwrap();
            assert_eq!(models.len(), 2, "one lost call, one allowed again");
            assert_eq!(
                models[0].payload(),
                models[1].payload(),
                "the same request, under the same id"
            );
        }
        let (parks,): (i64,) = sqlx::query_as("SELECT count(*) FROM activities WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(parks, 1, "no duplicate activity");

        // The connection drops again on the follow-up, twice. The park is
        // allowed once in this process; the second failure stays stalled,
        // so a broken model is not called on every poll.
        drops.store(2, Ordering::SeqCst);
        let first_revision = revision_of(&pool, &first_activity).await;
        let answer = json!({
            "requestKey": "attempt-1",
            "activityRevisionId": first_revision,
            "response": "the discounted sum of future rewards"
        });
        let (status, _) = call(&app, "POST", "/api/attempts", user, Some(answer)).await;
        assert_eq!(status, StatusCode::CREATED);
        let stalled = settled(&app, user, &ws).await;
        assert_eq!(stalled["operator"], "stalled", "{stalled}");
        assert_eq!(stalled["families"], json!(["call/model"]));
        let before = models.lock().unwrap().len();
        assert_eq!(before, 4, "wake, allowed once, both dropped");
        let again = settled(&app, user, &ws).await;
        assert_eq!(again["operator"], "stalled", "{again}");
        assert_eq!(
            models.lock().unwrap().len(),
            before,
            "a poll asks the model nothing more"
        );

        // A new process: the first read after reopen allows it once more,
        // the model answers, the follow-up lands, and nothing is duplicated.
        runtime.close(ws.parse().unwrap()).await.unwrap();
        let (app, runtime) = serve(
            &pool,
            scripted_dropping(models.clone(), vec![FOLLOW_UP], Arc::clone(&drops)),
        )
        .await;
        let resumed = settled(&app, user, &ws).await;
        assert_eq!(resumed["operator"], "waiting", "{resumed}");
        assert_ne!(resumed["currentActivityId"], first_activity);
        assert_eq!(
            models.lock().unwrap().len(),
            5,
            "allowed once more after the restart"
        );
        let (activities,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM activities WHERE user_id = $1")
                .bind(user)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(activities, 2);
        runtime.close(ws.parse().unwrap()).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_dead_process_lease_reads_as_unavailable_until_it_lapses(pool: PgPool) {
        let (app, runtime) = serve(&pool, scripted(Calls::default(), vec![FIRST])).await;
        let user = learner(&pool, "e@x.test").await;
        let (_, current) = call(&app, "GET", "/api/workspaces/current", user, None).await;
        let ws = current["id"].as_str().unwrap().to_string();
        call(
            &app,
            "POST",
            &format!("/api/workspaces/{ws}/goal"),
            user,
            Some(json!({ "intent": "Understand the return." })),
        )
        .await;
        let first = settled(&app, user, &ws).await;
        assert_eq!(first["operator"], "waiting", "{first}");
        runtime.close(ws.parse().unwrap()).await.unwrap();

        // The process died holding the lease: the row still names it, with
        // time left. A new process reads the page as unavailable, not 500.
        sqlx::query(
            "UPDATE workspace_sessions SET owner_lease = gen_random_uuid(),
             lease_until = now() + interval '1 minute' WHERE workspace_id = $1::uuid",
        )
        .bind(&ws)
        .execute(&pool)
        .await
        .unwrap();
        let (app, runtime) = serve(&pool, scripted(Calls::default(), vec![])).await;
        let (status, body) = call(&app, "GET", &format!("/api/workspaces/{ws}"), user, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["operator"], "unavailable", "{body}");
        assert_eq!(body["goal"]["intent"], "Understand the return.");

        // The lease lapses; the same read reopens the record and finds the wait.
        sqlx::query("UPDATE workspace_sessions SET lease_until = now() - interval '1 second' WHERE workspace_id = $1::uuid")
            .bind(&ws)
            .execute(&pool)
            .await
            .unwrap();
        let after = settled(&app, user, &ws).await;
        assert_eq!(after["operator"], "waiting", "{after}");
        assert_eq!(after["currentActivityId"], first["currentActivityId"]);
        runtime.close(ws.parse().unwrap()).await.unwrap();
    }
}
