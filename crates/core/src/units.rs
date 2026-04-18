//! Explicit unit types. Internal canonical length unit is millimeters.
//! Geometry math runs in f64; conversion to f32 happens at render boundaries only.

use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

pub type Vec2d = DVec2;
pub type Vec3d = DVec3;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct LengthMm(pub f64);

impl LengthMm {
    pub const ZERO: Self = Self(0.0);
    pub const fn new(mm: f64) -> Self {
        Self(mm)
    }
    pub fn as_f64(self) -> f64 {
        self.0
    }
    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AngleRad(pub f64);

impl AngleRad {
    pub const ZERO: Self = Self(0.0);
    pub fn from_degrees(deg: f64) -> Self {
        Self(deg.to_radians())
    }
    pub fn to_degrees(self) -> f64 {
        self.0.to_degrees()
    }
}
