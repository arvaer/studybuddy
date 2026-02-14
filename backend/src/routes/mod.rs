pub mod extractor;

mod auth;
mod concepts;
mod health;
mod llm;
mod notes;
mod progress;
mod questions;
mod quiz_sessions;
mod reinforcement_units;
mod resources;
mod settings;
mod study_sessions;
mod topics;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .nest(
            "/api",
            Router::new()
                .merge(topics::router())
                .merge(concepts::router())
                .merge(reinforcement_units::router())
                .merge(questions::router())
                .merge(quiz_sessions::router())
                .merge(study_sessions::router())
                .merge(notes::router())
                .merge(resources::router())
                .merge(progress::router())
                .merge(settings::router())
                .merge(llm::router()),
        )
}
