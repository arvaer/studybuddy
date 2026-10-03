//! The operator: StudyBuddy's Capsule Core host (#18). Phase 2 build plan,
//! `docs/phase-2-build.md`. This crate holds sessions; it writes no learning
//! records itself (providers in #19 do that through the application).

pub mod host;

pub use host::{OperatorError, OperatorHost};
