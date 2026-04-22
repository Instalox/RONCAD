//! A 2D sketch bound to a workplane. Owns its entities and persistent
//! dimensions; constraints and solving land later.

use glam::DVec2;
use roncad_core::ids::{ConstraintId, SketchDimensionId, SketchEntityId, WorkplaneId};
use slotmap::SlotMap;

use crate::arc_sample_points;
use crate::constraint::Constraint;
use crate::sketch_dimension::SketchDimension;
use crate::sketch_entity::SketchEntity;

#[derive(Debug, Clone)]
pub struct Sketch {
    pub name: String,
    pub workplane: WorkplaneId,
    pub entities: SlotMap<SketchEntityId, SketchEntity>,
    pub dimensions: SlotMap<SketchDimensionId, SketchDimension>,
    pub constraints: SlotMap<ConstraintId, Constraint>,
}

impl Sketch {
    pub fn new(name: impl Into<String>, workplane: WorkplaneId) -> Self {
        Self {
            name: name.into(),
            workplane,
            entities: SlotMap::with_key(),
            dimensions: SlotMap::with_key(),
            constraints: SlotMap::with_key(),
        }
    }

    pub fn add(&mut self, entity: SketchEntity) -> SketchEntityId {
        self.entities.insert(entity)
    }

    pub fn remove(&mut self, id: SketchEntityId) -> Option<SketchEntity> {
        let removed = self.entities.remove(id);
        if removed.is_some() {
            self.constraints
                .retain(|_, c| !c.referenced_entities().contains(&id));
        }
        removed
    }

    pub fn iter(&self) -> impl Iterator<Item = (SketchEntityId, &SketchEntity)> {
        self.entities.iter()
    }

    pub fn add_dimension(&mut self, dimension: SketchDimension) -> SketchDimensionId {
        self.dimensions.insert(dimension)
    }

    pub fn iter_dimensions(&self) -> impl Iterator<Item = (SketchDimensionId, &SketchDimension)> {
        self.dimensions.iter()
    }

    pub fn add_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        self.constraints.insert(constraint)
    }

    pub fn remove_constraint(&mut self, id: ConstraintId) -> Option<Constraint> {
        self.constraints.remove(id)
    }

    pub fn iter_constraints(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints.iter()
    }

    pub fn bounds(&self) -> Option<(DVec2, DVec2)> {
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        let mut has_any = false;

        for (_, entity) in self.iter() {
            let points: Vec<DVec2> = match entity {
                SketchEntity::Point { p } => vec![*p],
                SketchEntity::Line { a, b } => vec![*a, *b],
                SketchEntity::Rectangle { corner_a, corner_b } => {
                    vec![corner_a.min(*corner_b), corner_a.max(*corner_b)]
                }
                SketchEntity::Circle { center, radius } => vec![
                    *center + DVec2::new(-radius, -radius),
                    *center + DVec2::new(*radius, *radius),
                ],
                SketchEntity::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep_angle,
                } => arc_sample_points(
                    *center,
                    *radius,
                    *start_angle,
                    *sweep_angle,
                    std::f64::consts::PI / 48.0,
                ),
            };

            for point in points {
                min = min.min(point);
                max = max.max(point);
                has_any = true;
            }
        }

        has_any.then_some((min, max))
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::Sketch;
    use crate::SketchEntity;

    #[test]
    fn bounds_cover_circle_and_arc_extents() {
        let mut sketch = Sketch::new("Sketch", slotmap::KeyData::default().into());
        sketch.add(SketchEntity::Circle {
            center: dvec2(5.0, 2.0),
            radius: 3.0,
        });
        sketch.add(SketchEntity::Arc {
            center: dvec2(-4.0, -1.0),
            radius: 2.0,
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        });

        let (min, max) = sketch.bounds().expect("bounds");

        assert!(min.x <= -4.0);
        assert!(min.y <= -1.0);
        assert!(max.x >= 8.0);
        assert!(max.y >= 5.0);
    }
}
