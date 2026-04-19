//! The authoritative Project model: workplanes, sketches, and now body/feature
//! state for persistent operations like extrusion.

use roncad_core::ids::{BodyId, FeatureId, SketchId, WorkplaneId};
use slotmap::SlotMap;

use crate::body::Body;
use crate::feature::{ExtrudeFeature, Feature};
use crate::sketch::Sketch;
use crate::workplane::Workplane;
use crate::SketchProfile;

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub workplanes: SlotMap<WorkplaneId, Workplane>,
    pub sketches: SlotMap<SketchId, Sketch>,
    pub bodies: SlotMap<BodyId, Body>,
    pub features: SlotMap<FeatureId, Feature>,
    pub active_sketch: Option<SketchId>,
    next_body_serial: usize,
    next_feature_serial: usize,
}

impl Default for Project {
    fn default() -> Self {
        Self::new_untitled()
    }
}

impl Project {
    pub fn new_untitled() -> Self {
        let mut workplanes = SlotMap::with_key();
        let xy = workplanes.insert(Workplane::xy());
        workplanes.insert(Workplane::xz());
        workplanes.insert(Workplane::yz());

        let mut sketches = SlotMap::with_key();
        let first = sketches.insert(Sketch::new("Sketch 1", xy));

        Self {
            name: "Untitled".to_string(),
            workplanes,
            sketches,
            bodies: SlotMap::with_key(),
            features: SlotMap::with_key(),
            active_sketch: Some(first),
            next_body_serial: 1,
            next_feature_serial: 1,
        }
    }

    pub fn active_sketch(&self) -> Option<&Sketch> {
        self.active_sketch.and_then(|id| self.sketches.get(id))
    }

    pub fn active_sketch_mut(&mut self) -> Option<&mut Sketch> {
        match self.active_sketch {
            Some(id) => self.sketches.get_mut(id),
            None => None,
        }
    }

    pub fn sketch_workplane(&self, sketch_id: SketchId) -> Option<&Workplane> {
        let sketch = self.sketches.get(sketch_id)?;
        self.workplanes.get(sketch.workplane)
    }

    pub fn active_workplane(&self) -> Option<&Workplane> {
        let sketch_id = self.active_sketch?;
        self.sketch_workplane(sketch_id)
    }

    pub fn feature_world_bounds(&self, feature: &Feature) -> Option<(glam::DVec3, glam::DVec3)> {
        let plane = feature
            .source_sketch()
            .and_then(|sketch_id| self.sketch_workplane(sketch_id))
            .or_else(|| self.workplanes.values().next())?;
        let (min_local, max_local) = feature.bounds_3d();
        Some(plane.local_bounds_to_world_bounds(min_local, max_local))
    }

    pub fn extrude_profile(
        &mut self,
        sketch: SketchId,
        profile: SketchProfile,
        distance_mm: f64,
    ) -> Option<(BodyId, FeatureId)> {
        if !self.sketches.contains_key(sketch) || !distance_mm.is_finite() || distance_mm <= 0.0 {
            return None;
        }

        let body_name = self.allocate_body_name();
        let feature_name = self.allocate_feature_name();
        let body_id = self.bodies.insert(Body::new(body_name));
        let feature_id = self.features.insert(Feature::Extrude(ExtrudeFeature::new(
            feature_name,
            body_id,
            Some(sketch),
            profile,
            distance_mm,
        )));
        self.bodies.get_mut(body_id)?.push_feature(feature_id);
        Some((body_id, feature_id))
    }

    pub fn delete_body(&mut self, body_id: BodyId) -> bool {
        let Some(body) = self.bodies.remove(body_id) else {
            return false;
        };
        for feature_id in body.features {
            self.features.remove(feature_id);
        }
        true
    }

    pub fn clear_feature_sketch_source(&mut self, sketch_id: SketchId) {
        for (_, feature) in self.features.iter_mut() {
            feature.clear_source_sketch_if_matches(sketch_id);
        }
    }

    pub fn body_features(
        &self,
        body_id: BodyId,
    ) -> impl Iterator<Item = (FeatureId, &Feature)> + '_ {
        let feature_ids = self
            .bodies
            .get(body_id)
            .map(|body| body.features.clone())
            .unwrap_or_default();

        feature_ids.into_iter().filter_map(|feature_id| {
            self.features
                .get(feature_id)
                .map(|feature| (feature_id, feature))
        })
    }

    pub fn body_volume_mm3(&self, body_id: BodyId) -> f64 {
        self.body_features(body_id)
            .map(|(_, feature)| feature.volume_mm3())
            .sum()
    }

    fn allocate_body_name(&mut self) -> String {
        let name = format!("Body {}", self.next_body_serial);
        self.next_body_serial += 1;
        name
    }

    fn allocate_feature_name(&mut self) -> String {
        let name = format!("Extrude {}", self.next_feature_serial);
        self.next_feature_serial += 1;
        name
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::Project;
    use crate::SketchProfile;

    #[test]
    fn extrude_profile_creates_body_and_feature() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("sketch");

        let (body, feature) = project
            .extrude_profile(
                sketch,
                SketchProfile::Polygon {
                    points: vec![
                        dvec2(0.0, 0.0),
                        dvec2(10.0, 0.0),
                        dvec2(10.0, 5.0),
                        dvec2(0.0, 5.0),
                    ],
                },
                12.0,
            )
            .expect("extrusion");

        assert_eq!(project.bodies.len(), 1);
        assert_eq!(project.features.len(), 1);
        assert_eq!(project.bodies[body].name, "Body 1");
        assert_eq!(project.bodies[body].feature_count(), 1);
        assert_eq!(project.features[feature].name(), "Extrude 1");
        assert_eq!(project.body_volume_mm3(body), 600.0);
    }

    #[test]
    fn deleting_body_removes_attached_features() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("sketch");
        let (body, _) = project
            .extrude_profile(
                sketch,
                SketchProfile::Circle {
                    center: dvec2(2.0, 2.0),
                    radius: 4.0,
                },
                3.0,
            )
            .expect("extrusion");

        assert!(project.delete_body(body));

        assert!(project.bodies.is_empty());
        assert!(project.features.is_empty());
    }
}
