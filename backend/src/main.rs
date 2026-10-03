mod config;
mod error;
mod llm;
mod routes;
mod state;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use http::{header, Method};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub use config::Config;
pub use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load backend/.env if present (ignored by git; see docs/development.md).
    let _ = dotenvy::dotenv();

    // Configuration: every variable is read here, and required ones abort
    // startup with an error that names the variable but never its value.
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("configuration error: {e}");
            std::process::exit(2);
        }
    };

    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.log_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database
    let pool = infra::db::create_pool(&config.database_url).await?;

    // `lugia audit-artifacts`: compare the artifact catalog with the blob
    // directory and exit (#12). Reports only, never deletes, never migrates.
    if std::env::args().nth(1).as_deref() == Some("audit-artifacts") {
        let store = app::services::artifact::ArtifactStore::new(
            infra::repositories::artifact::PgArtifactRepository::new(pool),
            config.uploads_dir,
        );
        let report = store.audit().await?;
        for a in &report.missing {
            println!("missing {a}");
        }
        for a in &report.orphans {
            println!("orphan  {a}");
        }
        for p in &report.parts {
            println!("part    {}", p.display());
        }
        println!(
            "{} missing, {} orphan, {} part",
            report.missing.len(),
            report.orphans.len(),
            report.parts.len()
        );
        std::process::exit(if report.missing.is_empty() { 0 } else { 1 });
    }

    infra::db::run_migrations(&pool).await?;
    tracing::info!("Migrations applied");

    // CORS — allow frontend origin with credentials (cookies)
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin.clone())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    // Model access: fixed here, never from a request.
    let llm = match config.llm {
        Some(settings) => {
            tracing::info!(provider = settings.provider.name(), model = %settings.model, "LLM provider configured");
            Some(std::sync::Arc::new(llm::LlmClient::new(settings)?))
        }
        None => {
            tracing::info!("No LLM provider configured; /api/llm/proxy answers 503");
            None
        }
    };

    // The operator's effect endpoints (19a): served only with a secret,
    // which the embedded owner's providers present over loopback.
    let operator_secret = config.operator_secret.map(std::sync::Arc::from);
    if operator_secret.is_some() {
        tracing::info!("Operator effect endpoints served at /internal/effects");
    } else {
        tracing::info!("No OPERATOR_SECRET; /internal/effects answers 404");
    }

    let port = config.port;
    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        uploads_dir: config.uploads_dir,
        cookie_secure: config.cookie_secure,
        llm,
        auth_limiter: std::sync::Arc::new(app::services::rate_limit::AuthLimiter::new(config.auth_limits)),
        operator_secret,
    };

    let app = Router::new()
        .merge(routes::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // Connect info gives the login/signup limiter the peer address (#37).
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}
