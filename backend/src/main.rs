mod config;
mod error;
mod llm;
mod routes;
mod runtime;
mod state;
mod wakes;

use axum::Router;
use http::{header, Method};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
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

    // `lugia wakes <workspace-id>`: print a workspace's measured wakes
    // (21d) and exit. Reads only.
    if std::env::args().nth(1).as_deref() == Some("wakes") {
        let workspace: uuid::Uuid = std::env::args()
            .nth(2)
            .ok_or("usage: lugia wakes <workspace-id>")?
            .parse()?;
        wakes::print(&pool, workspace).await?;
        return Ok(());
    }

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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
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

    // The operator's effect endpoints (19a): the bearer secret the embedded
    // owner's providers present over loopback. Process-local, so one is
    // drawn here when OPERATOR_SECRET is unset (20a).
    let operator_secret: std::sync::Arc<str> = match config.operator_secret {
        Some(secret) => {
            tracing::info!(
                "Operator effect endpoints served at /internal/effects with OPERATOR_SECRET"
            );
            secret.into()
        }
        None => {
            tracing::info!(
                "Operator effect endpoints served at /internal/effects with a per-process secret"
            );
            format!(
                "{}{}",
                uuid::Uuid::new_v4().simple(),
                uuid::Uuid::new_v4().simple()
            )
            .into()
        }
    };

    // The operator's model (19b): the in-process Claude adapter, keyed by
    // ANTHROPIC_API_KEY as capsule-corp is. The key is never logged.
    let operator_model = config
        .anthropic_api_key
        .map(|key| operator::Model::new(key, config.operator_model.clone()));
    match &operator_model {
        Some(model) => tracing::info!(
            model = model.name(),
            "Operator model configured (in-process Claude adapter)"
        ),
        None => tracing::info!("No ANTHROPIC_API_KEY; the operator cannot think"),
    }

    // The program the operator runs (19c): compiled here so a broken capsule
    // fails the process, and its definition address logged, which pins what
    // source this process runs.
    let (capsule_name, capsule_address) = operator::capsule::check()?;
    tracing::info!(capsule = %capsule_name, definition = %capsule_address, "Learning capsule compiled");

    // The operator at run time (20a): sessions on demand, the model as
    // configured, publication through this process's own endpoint.
    let port = config.port;
    let installer: runtime::Installer = {
        let model = operator_model.clone();
        std::sync::Arc::new(
            move |session: &mut capsule_corp::sdk::Session<operator::host::Record>,
                  wakes: &wakes::Wakes| {
                if let Some(model) = &model {
                    // The adapter, measured: one `wakes` row per call (21d).
                    let mut claude = model.provider();
                    session.provide(
                        operator::model::FAMILY,
                        wakes.observe(move |effect: &capsule_corp::sdk::Effect| {
                            capsule_corp::sdk::Provider::serve(&mut claude, effect)
                        }),
                    );
                }
            },
        )
    };
    let runtime = std::sync::Arc::new(runtime::OperatorRuntime::new(
        pool.clone(),
        config.database_url.clone(),
        format!("http://127.0.0.1:{port}"),
        std::sync::Arc::clone(&operator_secret),
        installer,
    ));
    {
        let runtime = std::sync::Arc::clone(&runtime);
        tokio::spawn(async move {
            let mut every = tokio::time::interval(runtime::LEASE_RENEWAL);
            loop {
                every.tick().await;
                if let Err(error) = runtime.renew_leases().await {
                    tracing::warn!("lease renewal failed: {error}");
                }
            }
        });
    }

    let state = AppState {
        pool,
        jwt_secret: config.jwt_secret,
        uploads_dir: config.uploads_dir,
        cookie_secure: config.cookie_secure,
        llm,
        auth_limiter: std::sync::Arc::new(app::services::rate_limit::AuthLimiter::new(
            config.auth_limits,
        )),
        operator_secret: Some(operator_secret),
        operator_model,
        runtime,
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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
