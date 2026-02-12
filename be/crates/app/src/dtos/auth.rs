use serde::{Deserialize, Serialize};
use validator::Validate;

use domain::entities::User;

// ─── Requests ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    #[validate(email(message = "invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 1, max = 100, message = "display name required"))]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "invalid email address"))]
    pub email: String,

    #[validate(length(min = 1, message = "password required"))]
    pub password: String,
}

// ─── Responses ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user:          UserResponse,
    pub access_token:  String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id:           String,
    pub email:        String,
    pub display_name: String,
    pub avatar:       Option<String>,
    pub created_at:   String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id:           u.id.to_string(),
            email:        u.email,
            display_name: u.display_name,
            avatar:       u.avatar,
            created_at:   u.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,  // user id
    pub exp: i64,     // expiry unix ts
    pub iat: i64,     // issued at
}
