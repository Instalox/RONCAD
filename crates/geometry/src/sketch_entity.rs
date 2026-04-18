//! Sketch entities live in their workplane's local UV space (mm).
//! Milestone 2 ships Point, Line, Rectangle, Circle — constraints come later.

use glam::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SketchEntity {
    Point {
        p: DVec2,
    },
    Line {
        a: DVec2,
        b: DVec2,
    },
    Rectangle {
        corner_a: DVec2,
        corner_b: DVec2,
    },
    Circle {
        center: DVec2,
        radius: f64,
    },
}

impl SketchEntity {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Point { .. } => "Point",
            Self::Line { .. } => "Line",
            Self::Rectangle { .. } => "Rectangle",
            Self::Circle { .. } => "Circle",
        }
    }
}
