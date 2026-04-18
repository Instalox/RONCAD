//! Persistent sketch dimensions. Milestone 3 starts with distance dimensions
//! stored directly on the sketch before constraints and solving land.

use glam::DVec2;

#[derive(Debug, Clone, PartialEq)]
pub enum SketchDimension {
    Distance {
        start: DVec2,
        end: DVec2,
    },
}

impl SketchDimension {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Distance { .. } => "Distance",
        }
    }
}
