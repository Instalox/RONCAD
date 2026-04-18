//! A work plane anchoring 2D sketches inside 3D space.
//! Milestone 2 only uses the world XY plane; origin/u/v generalize later.

use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workplane {
    pub name: String,
    pub origin: DVec3,
    pub u: DVec3,
    pub v: DVec3,
}

impl Workplane {
    pub fn xy() -> Self {
        Self {
            name: "XY".into(),
            origin: DVec3::ZERO,
            u: DVec3::X,
            v: DVec3::Y,
        }
    }

    pub fn normal(&self) -> DVec3 {
        self.u.cross(self.v).normalize()
    }
}
