//! Request validation (#36). Every request DTO declares its constraints with
//! `validator::Validate`; this is the one place that turns a failed check into
//! an `AppError::Validation`, so every service entry point enforces what its
//! DTO declares and the HTTP layer answers 422 before any row is written.

use validator::{Validate, ValidationErrors};

use crate::errors::AppError;

/// Enforce a DTO's declared constraints. Call it first in every service entry
/// point that takes a `Validate` request.
pub fn validated<T: Validate>(req: &T) -> Result<(), AppError> {
    req.validate().map_err(validation_error)
}

/// Flatten validator output into one message. Field names only; never values.
pub fn validation_error(errs: ValidationErrors) -> AppError {
    let mut parts: Vec<String> = errs
        .field_errors()
        .iter()
        .flat_map(|(field, es)| {
            es.iter().map(move |e| match &e.message {
                Some(m) => m.to_string(),
                None => format!("{field} is invalid"),
            })
        })
        .collect();
    parts.sort();
    AppError::Validation(parts.join("; "))
}
