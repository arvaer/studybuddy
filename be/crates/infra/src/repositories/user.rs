use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use domain::entities::User;
use domain::errors::DomainError;
use domain::repository_traits::UserRepository;

#[derive(Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PgUserRepository {
    async fn create(
        &self,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<User, DomainError> {
        // pgcrypto: crypt(password, gen_salt('bf', 12)) hashes with bcrypt cost 12
        let row = sqlx::query!(
            r#"
            INSERT INTO users (email, password_hash, display_name)
            VALUES ($1, crypt($2, gen_salt('bf', 12)), $3)
            RETURNING id, email, display_name, avatar, created_at
            "#,
            email,
            password,
            display_name,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.code().as_deref() == Some("23505") => {
                DomainError::Conflict(format!("email already registered: {email}"))
            }
            other => DomainError::Repository(other.to_string()),
        })?;

        Ok(User {
            id:           row.id,
            email:        row.email,
            display_name: row.display_name,
            avatar:       row.avatar,
            created_at:   row.created_at,
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<User, DomainError> {
        sqlx::query_as!(
            User,
            "SELECT id, email, display_name, avatar, created_at FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("user {id}")))
    }

    async fn find_by_email(&self, email: &str) -> Result<User, DomainError> {
        sqlx::query_as!(
            User,
            "SELECT id, email, display_name, avatar, created_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound(format!("user with email {email}")))
    }

    async fn verify_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<User, DomainError> {
        // pgcrypto: crypt(entered_password, stored_hash) == stored_hash when correct
        let row = sqlx::query!(
            r#"
            SELECT id, email, display_name, avatar, created_at
            FROM users
            WHERE email = $1
              AND password_hash = crypt($2, password_hash)
            "#,
            email,
            password,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or(DomainError::Unauthorized)?;

        Ok(User {
            id:           row.id,
            email:        row.email,
            display_name: row.display_name,
            avatar:       row.avatar,
            created_at:   row.created_at,
        })
    }

    async fn store_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            user_id,
            token_hash,
            expires_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }

    async fn validate_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Uuid, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT user_id FROM refresh_tokens
            WHERE token_hash = $1 AND expires_at > now()
            "#,
            token_hash,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?
        .ok_or(DomainError::Unauthorized)?;

        Ok(row.user_id)
    }

    async fn revoke_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE token_hash = $1",
            token_hash,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }

    async fn revoke_all_refresh_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            "DELETE FROM refresh_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(())
    }
}
