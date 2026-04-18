//! Typed error surface for the core layer. Downstream crates add their own.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("entity not found")]
    NotFound,
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("state violation: {0}")]
    StateViolation(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
