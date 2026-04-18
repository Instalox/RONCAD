//! Typed identifier handles for domain entities.
//! Keeps us from passing raw integers or strings across module boundaries.

use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

new_key_type! {
    pub struct SketchId;
    pub struct SketchEntityId;
    pub struct ConstraintId;
    pub struct WorkplaneId;
    pub struct BodyId;
    pub struct FeatureId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolId(pub &'static str);

impl ToolId {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}
