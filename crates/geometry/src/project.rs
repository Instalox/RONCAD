//! The authoritative Project model: workplanes, sketches, and now body/feature
//! state for persistent operations like extrusion.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use roncad_core::ids::{BodyId, FeatureId, SketchId, WorkplaneId};
use slotmap::SlotMap;

use crate::body::Body;
use crate::feature::{ExtrudeFeature, Feature, RevolveFeature};
use crate::sketch::Sketch;
use crate::topology::SketchTopology;
use crate::workplane::Workplane;
use crate::SketchProfile;

static NEXT_RENDER_CACHE_KEY: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub workplanes: SlotMap<WorkplaneId, Workplane>,
    pub sketches: SlotMap<SketchId, Sketch>,
    pub bodies: SlotMap<BodyId, Body>,
    pub features: SlotMap<FeatureId, Feature>,
    pub active_sketch: Option<SketchId>,
    render_cache_key: u64,
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
            render_cache_key: next_render_cache_key(),
            next_body_serial: 1,
            next_feature_serial: 1,
        }
    }

    pub fn from_parts(
        name: impl Into<String>,
        workplanes: SlotMap<WorkplaneId, Workplane>,
        sketches: SlotMap<SketchId, Sketch>,
        bodies: SlotMap<BodyId, Body>,
        features: SlotMap<FeatureId, Feature>,
        active_sketch: Option<SketchId>,
    ) -> Self {
        let next_body_serial = next_body_serial(&bodies);
        let next_feature_serial = next_feature_serial(&features);

        let mut project = Self {
            name: name.into(),
            workplanes,
            sketches,
            bodies,
            features,
            active_sketch,
            render_cache_key: next_render_cache_key(),
            next_body_serial,
            next_feature_serial,
        };
        project.reattach_feature_profile_links();
        project
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

    pub fn render_cache_key(&self) -> u64 {
        self.render_cache_key
    }

    pub fn feature_world_bounds(&self, feature: &Feature) -> Option<(glam::DVec3, glam::DVec3)> {
        if !feature.is_profile_valid() {
            return None;
        }

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

        let profile_key = self
            .sketches
            .get(sketch)
            .map(SketchTopology::from_sketch)
            .and_then(|topology| {
                topology
                    .find_profile(&profile)
                    .map(|entry| entry.key.clone())
            });
        let source_sketch = profile_key.as_ref().map(|_| sketch);
        let body_name = self.allocate_body_name();
        let feature_name = self.allocate_feature_name("Extrude");
        let body_id = self.bodies.insert(Body::new(body_name));
        let mut feature =
            ExtrudeFeature::new(feature_name, body_id, source_sketch, profile, distance_mm);
        feature.profile_key = profile_key;
        let feature_id = self.features.insert(Feature::Extrude(feature));
        self.bodies.get_mut(body_id)?.push_feature(feature_id);
        Some((body_id, feature_id))
    }

    pub fn revolve_profile(
        &mut self,
        sketch: SketchId,
        profile: SketchProfile,
        axis_origin: glam::DVec2,
        axis_dir: glam::DVec2,
        angle_rad: f64,
    ) -> Option<(BodyId, FeatureId)> {
        if !self.sketches.contains_key(sketch)
            || axis_dir.length_squared() < 1e-6
            || angle_rad == 0.0
        {
            return None;
        }

        let profile_key = self
            .sketches
            .get(sketch)
            .map(SketchTopology::from_sketch)
            .and_then(|topology| {
                topology
                    .find_profile(&profile)
                    .map(|entry| entry.key.clone())
            });
        let source_sketch = profile_key.as_ref().map(|_| sketch);
        let body_name = self.allocate_body_name();
        let feature_name = self.allocate_feature_name("Revolve");
        let body_id = self.bodies.insert(Body::new(body_name));
        let mut feature = RevolveFeature::new(
            feature_name,
            body_id,
            source_sketch,
            profile,
            axis_origin,
            axis_dir.normalize(),
            angle_rad,
        );
        feature.profile_key = profile_key;
        let feature_id = self.features.insert(Feature::Revolve(feature));
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
        let mut dirty_bodies = HashSet::new();
        for (_, feature) in self.features.iter_mut() {
            let body_id = feature.body();
            let was_linked = feature.source_sketch() == Some(sketch_id);
            feature.clear_source_sketch_if_matches(sketch_id);
            if was_linked {
                dirty_bodies.insert(body_id);
            }
        }
        for body_id in dirty_bodies {
            if let Some(body) = self.bodies.get_mut(body_id) {
                body.bump_mesh_revision();
            }
        }
    }

    pub fn rebuild_features_for_sketch(&mut self, sketch_id: SketchId) {
        let Some(sketch) = self.sketches.get(sketch_id) else {
            return;
        };
        let topology = SketchTopology::from_sketch(sketch);
        let mut dirty_bodies = HashSet::new();
        for (_, feature) in self.features.iter_mut() {
            if feature.source_sketch() == Some(sketch_id) {
                dirty_bodies.insert(feature.body());
                feature.rebuild_from_topology(&topology);
            }
        }
        for body_id in dirty_bodies {
            if let Some(body) = self.bodies.get_mut(body_id) {
                body.bump_mesh_revision();
            }
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
            .filter(|(_, feature)| feature.is_profile_valid())
            .map(|(_, feature)| feature.volume_mm3())
            .sum()
    }

    fn allocate_body_name(&mut self) -> String {
        let name = format!("Body {}", self.next_body_serial);
        self.next_body_serial += 1;
        name
    }

    fn allocate_feature_name(&mut self, prefix: &str) -> String {
        let name = format!("{} {}", prefix, self.next_feature_serial);
        self.next_feature_serial += 1;
        name
    }

    fn reattach_feature_profile_links(&mut self) {
        let sketches = &self.sketches;
        let mut topology_cache = HashMap::<SketchId, SketchTopology>::new();

        for (_, feature) in self.features.iter_mut() {
            let Some(sketch_id) = feature.source_sketch() else {
                feature.attach_profile_key(None);
                continue;
            };

            let topology = topology_cache.entry(sketch_id).or_insert_with(|| {
                sketches
                    .get(sketch_id)
                    .map(SketchTopology::from_sketch)
                    .unwrap_or_default()
            });
            let key = topology
                .find_profile(feature.profile())
                .map(|entry| entry.key.clone());
            feature.attach_profile_key(key);
        }
    }
}

fn next_render_cache_key() -> u64 {
    NEXT_RENDER_CACHE_KEY.fetch_add(1, Ordering::Relaxed)
}

fn next_body_serial(bodies: &SlotMap<BodyId, Body>) -> usize {
    let next_from_names = bodies
        .values()
        .filter_map(|body| parse_prefixed_serial(&body.name, "Body"))
        .max()
        .map(|serial| serial + 1)
        .unwrap_or(1);

    next_from_names.max(bodies.len() + 1)
}

fn next_feature_serial(features: &SlotMap<FeatureId, Feature>) -> usize {
    let next_from_names = features
        .values()
        .filter_map(|feature| parse_trailing_serial(feature.name()))
        .max()
        .map(|serial| serial + 1)
        .unwrap_or(1);

    next_from_names.max(features.len() + 1)
}

fn parse_prefixed_serial(name: &str, prefix: &str) -> Option<usize> {
    name.strip_prefix(prefix)?
        .strip_prefix(' ')?
        .parse::<usize>()
        .ok()
}

fn parse_trailing_serial(name: &str) -> Option<usize> {
    name.rsplit_once(' ')?.1.parse::<usize>().ok()
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

    #[test]
    fn rebuild_features_for_sketch_refreshes_linked_profile() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let circle =
            project
                .sketches
                .get_mut(sketch_id)
                .expect("sketch")
                .add(crate::SketchEntity::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 2.0,
                });
        let profile = SketchProfile::Circle {
            center: dvec2(5.0, 5.0),
            radius: 2.0,
        };
        let (_, feature_id) = project
            .extrude_profile(sketch_id, profile, 10.0)
            .expect("extrude");

        if let crate::SketchEntity::Circle { radius, .. } = project.sketches[sketch_id]
            .entities
            .get_mut(circle)
            .expect("circle")
        {
            *radius = 4.0;
        }

        let body_id = project.features[feature_id].body();
        let previous_revision = project.bodies[body_id].mesh_revision();
        project.rebuild_features_for_sketch(sketch_id);

        let feature = project.features.get(feature_id).expect("feature");
        assert!(feature.is_profile_valid());
        assert_eq!(feature.area_mm2(), std::f64::consts::PI * 16.0);
        assert!(project.bodies[body_id].mesh_revision() > previous_revision);
    }

    #[test]
    fn rebuild_features_for_sketch_invalidates_missing_profile() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let circle =
            project
                .sketches
                .get_mut(sketch_id)
                .expect("sketch")
                .add(crate::SketchEntity::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 2.0,
                });
        let (_, feature_id) = project
            .extrude_profile(
                sketch_id,
                SketchProfile::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 2.0,
                },
                10.0,
            )
            .expect("extrude");

        project.sketches[sketch_id].remove(circle);
        project.rebuild_features_for_sketch(sketch_id);

        let feature = project.features.get(feature_id).expect("feature");
        assert!(!feature.is_profile_valid());
        assert_eq!(project.body_volume_mm3(feature.body()), 0.0);
    }
}
