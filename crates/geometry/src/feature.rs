use glam::{DVec2, DVec3};
use roncad_core::ids::{BodyId, SketchId};

use crate::{ProfileKey, SketchProfile, SketchTopology};

#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    Extrude(ExtrudeFeature),
    Revolve(RevolveFeature),
}

impl Feature {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Extrude(_) => "Extrude",
            Self::Revolve(_) => "Revolve",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Extrude(feature) => &feature.name,
            Self::Revolve(feature) => &feature.name,
        }
    }

    pub fn body(&self) -> BodyId {
        match self {
            Self::Extrude(feature) => feature.body,
            Self::Revolve(feature) => feature.body,
        }
    }

    pub fn source_sketch(&self) -> Option<SketchId> {
        match self {
            Self::Extrude(feature) => feature.source_sketch,
            Self::Revolve(feature) => feature.source_sketch,
        }
    }

    pub fn clear_source_sketch_if_matches(&mut self, sketch: SketchId) {
        match self {
            Self::Extrude(feature) if feature.source_sketch == Some(sketch) => {
                feature.source_sketch = None;
                feature.profile_key = None;
                feature.profile_valid = true;
            }
            Self::Revolve(feature) if feature.source_sketch == Some(sketch) => {
                feature.source_sketch = None;
                feature.profile_key = None;
                feature.profile_valid = true;
            }
            Self::Extrude(_) | Self::Revolve(_) => {}
        }
    }

    pub fn profile_key(&self) -> Option<&ProfileKey> {
        match self {
            Self::Extrude(feature) => feature.profile_key.as_ref(),
            Self::Revolve(feature) => feature.profile_key.as_ref(),
        }
    }

    pub fn attach_profile_key(&mut self, key: Option<ProfileKey>) {
        let is_linked = self.source_sketch().is_some();
        let is_valid = !is_linked || key.is_some();
        match self {
            Self::Extrude(feature) => {
                feature.profile_key = key;
                feature.profile_valid = is_valid;
            }
            Self::Revolve(feature) => {
                feature.profile_key = key;
                feature.profile_valid = is_valid;
            }
        }
    }

    pub fn is_profile_valid(&self) -> bool {
        match self {
            Self::Extrude(feature) => feature.profile_valid,
            Self::Revolve(feature) => feature.profile_valid,
        }
    }

    pub fn rebuild_from_topology(&mut self, topology: &SketchTopology) -> bool {
        let Some(key) = self.profile_key().cloned() else {
            match self {
                Self::Extrude(feature) => feature.profile_valid = false,
                Self::Revolve(feature) => feature.profile_valid = false,
            }
            return false;
        };

        let Some(profile) = topology
            .profile_by_key(&key)
            .map(|entry| entry.profile.clone())
        else {
            match self {
                Self::Extrude(feature) => feature.profile_valid = false,
                Self::Revolve(feature) => feature.profile_valid = false,
            }
            return false;
        };

        match self {
            Self::Extrude(feature) => {
                feature.profile = profile;
                feature.profile_valid = true;
            }
            Self::Revolve(feature) => {
                feature.profile = profile;
                feature.profile_valid = true;
            }
        }
        true
    }

    pub fn profile(&self) -> &SketchProfile {
        match self {
            Self::Extrude(feature) => &feature.profile,
            Self::Revolve(feature) => &feature.profile,
        }
    }

    pub fn distance_mm(&self) -> f64 {
        match self {
            Self::Extrude(feature) => feature.distance_mm,
            Self::Revolve(_) => 0.0,
        }
    }

    pub fn area_mm2(&self) -> f64 {
        if self.is_profile_valid() {
            self.profile().area()
        } else {
            0.0
        }
    }

    pub fn volume_mm3(&self) -> f64 {
        if !self.is_profile_valid() {
            return 0.0;
        }

        match self {
            Self::Extrude(feature) => self.area_mm2() * feature.distance_mm.abs(),
            Self::Revolve(feature) => {
                // Pappus's centroid theorem: V = A * 2*pi*r, where r is distance from centroid to axis
                // This is an approximation if the axis intersects the profile, but fine for now
                let centroid = self.profile().centroid();
                // Find distance from centroid to the axis line (axis_origin, axis_dir)
                let to_centroid = centroid - feature.axis_origin;
                // cross product in 2d gives distance to line if direction is normalized
                let r =
                    (to_centroid.x * feature.axis_dir.y - to_centroid.y * feature.axis_dir.x).abs();
                self.area_mm2() * feature.angle_rad.abs() * r
            }
        }
    }

    pub fn bounds_3d(&self) -> (DVec3, DVec3) {
        match self {
            Self::Extrude(feature) => {
                let (min_2d, max_2d) = profile_bounds(self.profile());
                let min_z = feature.distance_mm.min(0.0);
                let max_z = feature.distance_mm.max(0.0);
                (
                    DVec3::new(min_2d.x, min_2d.y, min_z),
                    DVec3::new(max_2d.x, max_2d.y, max_z),
                )
            }
            Self::Revolve(feature) => {
                // Simple conservative bounding box
                let (min_2d, max_2d) = profile_bounds(self.profile());
                // Find max radius from axis origin to any point in the bounding box
                let corners = [
                    min_2d,
                    DVec2::new(max_2d.x, min_2d.y),
                    max_2d,
                    DVec2::new(min_2d.x, max_2d.y),
                ];
                let mut max_r = 0.0_f64;
                for corner in corners {
                    let to_corner = corner - feature.axis_origin;
                    let r =
                        (to_corner.x * feature.axis_dir.y - to_corner.y * feature.axis_dir.x).abs();
                    max_r = max_r.max(r);
                }

                // For a full revolve, the shape sweeps through space.
                // A conservative bound is a cylinder along the axis.
                // Assuming axis is in the XY plane.
                let max_coord = max_r + min_2d.abs().max(max_2d.abs()).length();
                (
                    DVec3::new(-max_coord, -max_coord, -max_r),
                    DVec3::new(max_coord, max_coord, max_r),
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeFeature {
    pub name: String,
    pub body: BodyId,
    pub source_sketch: Option<SketchId>,
    pub profile_key: Option<ProfileKey>,
    pub profile_valid: bool,
    pub profile: SketchProfile,
    pub distance_mm: f64,
}

impl ExtrudeFeature {
    pub fn new(
        name: impl Into<String>,
        body: BodyId,
        source_sketch: Option<SketchId>,
        profile: SketchProfile,
        distance_mm: f64,
    ) -> Self {
        Self {
            name: name.into(),
            body,
            source_sketch,
            profile_key: None,
            profile_valid: true,
            profile,
            distance_mm,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevolveFeature {
    pub name: String,
    pub body: BodyId,
    pub source_sketch: Option<SketchId>,
    pub profile_key: Option<ProfileKey>,
    pub profile_valid: bool,
    pub profile: SketchProfile,
    pub axis_origin: DVec2,
    pub axis_dir: DVec2,
    pub angle_rad: f64,
}

impl RevolveFeature {
    pub fn new(
        name: impl Into<String>,
        body: BodyId,
        source_sketch: Option<SketchId>,
        profile: SketchProfile,
        axis_origin: DVec2,
        axis_dir: DVec2,
        angle_rad: f64,
    ) -> Self {
        Self {
            name: name.into(),
            body,
            source_sketch,
            profile_key: None,
            profile_valid: true,
            profile,
            axis_origin,
            axis_dir,
            angle_rad,
        }
    }
}

fn profile_bounds(profile: &SketchProfile) -> (DVec2, DVec2) {
    match profile {
        SketchProfile::Polygon { points } => {
            let mut min = DVec2::splat(f64::INFINITY);
            let mut max = DVec2::splat(f64::NEG_INFINITY);
            for point in points {
                min = min.min(*point);
                max = max.max(*point);
            }
            (min, max)
        }
        SketchProfile::Circle { center, radius } => (
            *center + DVec2::new(-radius, -radius),
            *center + DVec2::new(*radius, *radius),
        ),
    }
}
