use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use domain::repository_traits::UserRepository;

use crate::dtos::auth::{AuthResponse, LoginRequest, SignupRequest, TokenClaims, UserResponse};
use crate::errors::AppError;

pub struct AuthService<R: UserRepository> {
    user_repo:     R,
    jwt_secret:    String,
    access_expiry: Duration,  // e.g. 15 minutes
    refresh_expiry: Duration, // e.g. 7 days
}

impl<R: UserRepository> AuthService<R> {
    pub fn new(user_repo: R, jwt_secret: String) -> Self {
        Self {
            user_repo,
            jwt_secret,
            access_expiry:  Duration::minutes(15),
            refresh_expiry: Duration::days(7),
        }
    }

    pub async fn signup(&self, req: SignupRequest) -> Result<(AuthResponse, String), AppError> {
        let user = self
            .user_repo
            .create(&req.email, &req.password, &req.display_name)
            .await?;

        let access_token = self.mint_access_token(user.id)?;
        let refresh_raw  = self.generate_refresh_token();
        let refresh_hash = sha256_hex(&refresh_raw);

        self.user_repo
            .store_refresh_token(user.id, &refresh_hash, Utc::now() + self.refresh_expiry)
            .await?;

        Ok((
            AuthResponse {
                user:         UserResponse::from(user),
                access_token,
            },
            refresh_raw,
        ))
    }

    pub async fn login(&self, req: LoginRequest) -> Result<(AuthResponse, String), AppError> {
        let user = self
            .user_repo
            .verify_password(&req.email, &req.password)
            .await
            .map_err(|_| AppError::Unauthorized("invalid credentials".into()))?;

        let access_token = self.mint_access_token(user.id)?;
        let refresh_raw  = self.generate_refresh_token();
        let refresh_hash = sha256_hex(&refresh_raw);

        self.user_repo
            .store_refresh_token(user.id, &refresh_hash, Utc::now() + self.refresh_expiry)
            .await?;

        Ok((
            AuthResponse {
                user:         UserResponse::from(user),
                access_token,
            },
            refresh_raw,
        ))
    }

    pub async fn logout(&self, refresh_raw: Option<&str>) -> Result<(), AppError> {
        if let Some(raw) = refresh_raw {
            let hash = sha256_hex(raw);
            let _ = self.user_repo.revoke_refresh_token(&hash).await;
        }
        Ok(())
    }

    pub async fn refresh(&self, refresh_raw: &str) -> Result<(String, String), AppError> {
        let hash    = sha256_hex(refresh_raw);
        let user_id = self.user_repo.validate_refresh_token(&hash).await?;

        // Rotate: revoke old, issue new
        self.user_repo.revoke_refresh_token(&hash).await?;

        let access_token    = self.mint_access_token(user_id)?;
        let new_refresh_raw = self.generate_refresh_token();
        let new_hash        = sha256_hex(&new_refresh_raw);

        self.user_repo
            .store_refresh_token(user_id, &new_hash, Utc::now() + self.refresh_expiry)
            .await?;

        Ok((access_token, new_refresh_raw))
    }

    pub fn verify_access_token(&self, token: &str) -> Result<Uuid, AppError> {
        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        let data = decode::<TokenClaims>(token, &key, &Validation::default())
            .map_err(|e| AppError::Unauthorized(e.to_string()))?;

        let user_id = Uuid::parse_str(&data.claims.sub)
            .map_err(|_| AppError::Unauthorized("malformed token subject".into()))?;

        Ok(user_id)
    }

    // ─── Private helpers ────────────────────────────────────────────────────

    fn mint_access_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = TokenClaims {
            sub: user_id.to_string(),
            iat: now.timestamp(),
            exp: (now + self.access_expiry).timestamp(),
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Token(e.to_string()))
    }

    fn generate_refresh_token(&self) -> String {
        use std::fmt::Write;
        let bytes: [u8; 32] = std::array::from_fn(|_| rand_byte());
        bytes.iter().fold(String::with_capacity(64), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
    }
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn rand_byte() -> u8 {
    uuid::Uuid::new_v4().as_bytes()[0]
}
