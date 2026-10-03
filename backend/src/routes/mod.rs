pub mod extractor;

mod activities;
mod artifacts;
mod attempts;
mod auth;
mod concepts;
mod health;
mod internal;
mod llm;
mod notes;
mod progress;
mod questions;
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
        .merge(internal::router())
        .nest(
            "/api",
            Router::new()
                .merge(activities::router())
                .merge(artifacts::router())
                .merge(attempts::router())
                .merge(topics::router())
                .merge(concepts::router())
                .merge(reinforcement_units::router())
                .merge(questions::router())
                .merge(study_sessions::router())
                .merge(notes::router())
                .merge(resources::router())
                .merge(progress::router())
                .merge(settings::router())
                .merge(llm::router()),
        )
}
