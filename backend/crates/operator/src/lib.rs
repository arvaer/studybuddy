//! The operator: StudyBuddy's Capsule Core host (#18). Phase 2 build plan,
//! `docs/phase-2-build.md`. This crate holds sessions; it writes no learning
//! records itself (providers in #19 do that through the application).

pub mod capsule;
pub mod host;
pub mod model;
pub mod providers;
pub mod receipts;

pub use host::{OperatorError, OperatorHost, Reconciled, Settled};
pub use model::Model;
pub use receipts::{Receipt, Recorded};
