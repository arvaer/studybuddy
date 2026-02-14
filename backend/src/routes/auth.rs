use axum::{
    Router,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};

use app::dtos::auth::{LoginRequest, SignupRequest};
use app::services::auth::AuthService;
use infra::repositories::user::PgUserRepository;

use crate::error::HttpError;
use crate::routes::extractor::AuthUser;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/signup",  post(signup))
        .route("/api/auth/login",   post(login))
        .route("/api/auth/logout",  post(logout))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/me",      get(me))
}

async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<SignupRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    let (resp, refresh_token) = svc.signup(req).await?;

    let jar = set_cookies(jar, &resp.access_token, &refresh_token, &state);
    Ok((jar, Json(resp)))
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let svc = make_service(&state);
    let (resp, refresh_token) = svc.login(req).await?;

    let jar = set_cookies(jar, &resp.access_token, &refresh_token, &state);
    Ok((jar, Json(resp)))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, HttpError> {
    let svc    = make_service(&state);
    let raw    = jar.get("refresh_token").map(|c| c.value().to_string());
    svc.logout(raw.as_deref()).await?;

    let jar = jar
        .remove(Cookie::from("access_token"))
        .remove(Cookie::from("refresh_token"));

    Ok((jar, Json(serde_json::json!({ "ok": true }))))
}

async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, HttpError> {
    let refresh_raw = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| app::errors::AppError::Unauthorized("missing refresh token".into()))?;

    let svc = make_service(&state);
    let (new_access, new_refresh) = svc.refresh(&refresh_raw).await?;

    let jar = set_cookies(jar, &new_access, &new_refresh, &state);
    Ok((jar, Json(serde_json::json!({ "access_token": new_access }))))
}

async fn me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<impl IntoResponse, HttpError> {
    use infra::repositories::user::PgUserRepository;
    use domain::repository_traits::UserRepository;

    let repo = PgUserRepository::new(state.pool.clone());
    let user = repo.find_by_id(user_id).await?;

    use app::dtos::auth::UserResponse;
    Ok(Json(UserResponse::from(user)))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_service(state: &AppState) -> AuthService<PgUserRepository> {
    let repo = PgUserRepository::new(state.pool.clone());
    AuthService::new(repo, state.jwt_secret.clone())
}

fn set_cookies(
    jar: CookieJar,
    access_token: &str,
    refresh_token: &str,
    _state: &AppState,
) -> CookieJar {
    let access = Cookie::build(("access_token", access_token.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build();

    let refresh = Cookie::build(("refresh_token", refresh_token.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/api/auth")   // limit scope to auth endpoints
        .build();

    jar.add(access).add(refresh)
}
