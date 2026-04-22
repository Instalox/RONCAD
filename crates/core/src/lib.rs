//! Shared primitives and abstractions for RONCAD.
//! No UI, rendering, or geometry dependencies live here.

pub mod command;
pub mod constraint;
pub mod error;
pub mod event;
pub mod ids;
pub mod selection;
pub mod transaction;
pub mod units;

pub use constraint::{Constraint, EntityPoint};
pub use error::{CoreError, CoreResult};
