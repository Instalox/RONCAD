use glam::{DVec2, DVec3};
use roncad_core::ids::{BodyId, SketchId};

use crate::SketchProfile;

#[derive(Debug, Clone, PartialEq)]
pub enum Feature {
    Extrude(ExtrudeFeature),
}

impl Feature {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Extrude(_) => "Extrude",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Extrude(feature) => &feature.name,
        }
    }

    pub fn body(&self) -> BodyId {
        match self {
            Self::Extrude(feature) => feature.body,
        }
    }

    pub fn source_sketch(&self) -> Option<SketchId> {
        match self {
            Self::Extrude(feature) => feature.source_sketch,
        }
    }

    pub fn clear_source_sketch_if_matches(&mut self, sketch: SketchId) {
        match self {
            Self::Extrude(feature) if feature.source_sketch == Some(sketch) => {
                feature.source_sketch = None;
            }
            Self::Extrude(_) => {}
        }
    }

    pub fn profile(&self) -> &SketchProfile {
        match self {
            Self::Extrude(feature) => &feature.profile,
        }
    }

    pub fn distance_mm(&self) -> f64 {
        match self {
            Self::Extrude(feature) => feature.distance_mm,
        }
    }

    pub fn area_mm2(&self) -> f64 {
        self.profile().area()
    }

    pub fn volume_mm3(&self) -> f64 {
        self.area_mm2() * self.distance_mm().abs()
    }

    pub fn bounds_3d(&self) -> (DVec3, DVec3) {
        let (min_2d, max_2d) = profile_bounds(self.profile());
        let min_z = self.distance_mm().min(0.0);
        let max_z = self.distance_mm().max(0.0);
        (
            DVec3::new(min_2d.x, min_2d.y, min_z),
            DVec3::new(max_2d.x, max_2d.y, max_z),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeFeature {
    pub name: String,
    pub body: BodyId,
    pub source_sketch: Option<SketchId>,
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
            profile,
            distance_mm,
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
