use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde_json::json;
use uuid::Uuid;

use app::dtos::auth::TokenClaims;

use crate::state::AppState;

/// Extracts and validates the authenticated user from either:
///   1. `Authorization: Bearer <token>` header
///   2. `access_token` HttpOnly cookie
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = extract_raw_token(parts)
            .and_then(|token| verify_token(&token, &state.jwt_secret).map(AuthUser));
        async move { result }
    }
}

fn extract_raw_token(parts: &Parts) -> Result<String, AuthError> {
    // 1. Authorization: Bearer <token>
    if let Some(header) = parts.headers.get("Authorization") {
        if let Ok(value) = header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Ok(token.to_owned());
            }
        }
    }

    // 2. access_token cookie
    let jar = CookieJar::from_headers(&parts.headers);
    if let Some(cookie) = jar.get("access_token") {
        return Ok(cookie.value().to_owned());
    }

    Err(AuthError::MissingToken)
}

fn verify_token(token: &str, secret: &str) -> Result<Uuid, AuthError> {
    let key  = DecodingKey::from_secret(secret.as_bytes());
    let data = decode::<TokenClaims>(token, &key, &Validation::default())
        .map_err(|_| AuthError::InvalidToken)?;

    Uuid::parse_str(&data.claims.sub).map_err(|_| AuthError::InvalidToken)
}

// ─── Rejection type ──────────────────────────────────────────────────────────

pub enum AuthError {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let msg = match self {
            AuthError::MissingToken => "missing authentication token",
            AuthError::InvalidToken => "invalid or expired token",
        };
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
    }
}
