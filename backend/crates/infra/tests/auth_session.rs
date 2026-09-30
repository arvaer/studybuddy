//! Session behaviour of `AuthService` (#7): issuance, rotation, revocation,
//! and the request validation that the DTOs declare.

use app::dtos::auth::{LoginRequest, SignupRequest};
use app::errors::AppError;
use app::services::auth::AuthService;
use infra::repositories::user::PgUserRepository;
use sqlx::PgPool;

fn service(pool: &PgPool) -> AuthService<PgUserRepository> {
    AuthService::new(PgUserRepository::new(pool.clone()), "test-secret".into())
}

fn signup_req(email: &str, password: &str) -> SignupRequest {
    SignupRequest { email: email.into(), password: password.into(), display_name: "Ada".into() }
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_rotates_and_the_old_token_is_dead(pool: PgPool) {
    let svc = service(&pool);
    let (_, refresh1) = svc.signup(signup_req("a@example.com", "correct horse")).await.unwrap();

    let (_, refresh2) = svc.refresh(&refresh1).await.unwrap();
    assert_ne!(refresh1, refresh2);

    let again = svc.refresh(&refresh1).await;
    assert!(matches!(again, Err(AppError::Domain(domain::errors::DomainError::Unauthorized))), "{again:?}");

    svc.refresh(&refresh2).await.expect("the rotated token still works");
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_revokes_the_refresh_token(pool: PgPool) {
    let svc = service(&pool);
    let (_, refresh) = svc.signup(signup_req("a@example.com", "correct horse")).await.unwrap();

    svc.logout(Some(&refresh)).await.unwrap();

    assert!(svc.refresh(&refresh).await.is_err());
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn expired_refresh_token_is_rejected(pool: PgPool) {
    let svc = service(&pool);
    let (_, refresh) = svc.signup(signup_req("a@example.com", "correct horse")).await.unwrap();

    sqlx::query("UPDATE refresh_tokens SET expires_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();

    assert!(svc.refresh(&refresh).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn signup_enforces_declared_validation(pool: PgPool) {
    let svc = service(&pool);

    let short = svc.signup(signup_req("a@example.com", "short")).await;
    assert!(matches!(short, Err(AppError::Validation(ref m)) if m.contains("at least 8")), "{short:?}");

    let bad_email = svc.signup(signup_req("not-an-email", "correct horse")).await;
    assert!(matches!(bad_email, Err(AppError::Validation(_))), "{bad_email:?}");

    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users").fetch_one(&pool).await.unwrap();
    assert_eq!(users, 0, "nothing was written");
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_wrong_password_is_unauthorized_and_issues_nothing(pool: PgPool) {
    let svc = service(&pool);
    svc.signup(signup_req("a@example.com", "correct horse")).await.unwrap();

    let res = svc.login(LoginRequest { email: "a@example.com".into(), password: "wrong".into() }).await;
    assert!(matches!(res, Err(AppError::Unauthorized(_))), "{res:?}");

    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM refresh_tokens").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 1, "only the signup token exists");
}
