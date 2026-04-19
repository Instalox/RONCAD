//! A work plane anchoring 2D sketches inside 3D space.
//! Sketch entities live in plane-local 2D coordinates and are projected into
//! 3D through the workplane basis.

use glam::{DVec2, DVec3};
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

    pub fn xz() -> Self {
        Self {
            name: "XZ".into(),
            origin: DVec3::ZERO,
            u: DVec3::X,
            v: DVec3::Z,
        }
    }

    pub fn yz() -> Self {
        Self {
            name: "YZ".into(),
            origin: DVec3::ZERO,
            u: DVec3::Y,
            v: DVec3::Z,
        }
    }

    pub fn normal(&self) -> DVec3 {
        self.u.cross(self.v).normalize()
    }

    pub fn local_point(&self, point: DVec2) -> DVec3 {
        self.origin + self.u.normalize() * point.x + self.v.normalize() * point.y
    }

    pub fn local_position(&self, position: DVec3) -> DVec3 {
        self.origin
            + self.u.normalize() * position.x
            + self.v.normalize() * position.y
            + self.normal() * position.z
    }

    pub fn world_to_local(&self, world: DVec3) -> DVec2 {
        let delta = world - self.origin;
        DVec2::new(delta.dot(self.u.normalize()), delta.dot(self.v.normalize()))
    }

    pub fn local_bounds_to_world_bounds(
        &self,
        min_local: DVec3,
        max_local: DVec3,
    ) -> (DVec3, DVec3) {
        let corners = [
            DVec3::new(min_local.x, min_local.y, min_local.z),
            DVec3::new(max_local.x, min_local.y, min_local.z),
            DVec3::new(min_local.x, max_local.y, min_local.z),
            DVec3::new(max_local.x, max_local.y, min_local.z),
            DVec3::new(min_local.x, min_local.y, max_local.z),
            DVec3::new(max_local.x, min_local.y, max_local.z),
            DVec3::new(min_local.x, max_local.y, max_local.z),
            DVec3::new(max_local.x, max_local.y, max_local.z),
        ];

        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        for corner in corners {
            let world = self.local_position(corner);
            min = min.min(world);
            max = max.max(world);
        }
        (min, max)
    }
}
