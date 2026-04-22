//! Inferring constraints from freshly-inserted geometry.
//!
//! Runs after a sketch insert. Looks at the new entity relative to what's
//! already there and records any relationships that are obviously intended:
//! H/V lines, endpoints that land on another entity's endpoint or center.
//!
//! Tolerance is 1e-4 mm — well inside any real CAD workflow and loose enough
//! that snap-produced points reliably match despite float noise through
//! world/plane transforms.

use roncad_core::ids::SketchEntityId;

use crate::constraint::{resolve_entity_point, Constraint, EntityPoint};
use crate::sketch::Sketch;
use crate::sketch_entity::SketchEntity;

pub const INFERENCE_EPSILON: f64 = 1e-4;

/// Inspect `new_id` in the context of the rest of `sketch` and record any
/// constraints that follow from it.
pub fn infer_constraints(sketch: &mut Sketch, new_id: SketchEntityId) {
    let Some(entity) = sketch.entities.get(new_id).cloned() else {
        return;
    };

    let mut to_add = Vec::new();

    match entity {
        SketchEntity::Line { a, b } => {
            let dx = (a.x - b.x).abs();
            let dy = (a.y - b.y).abs();
            if dy < INFERENCE_EPSILON && dx >= INFERENCE_EPSILON {
                to_add.push(Constraint::Horizontal { entity: new_id });
            } else if dx < INFERENCE_EPSILON && dy >= INFERENCE_EPSILON {
                to_add.push(Constraint::Vertical { entity: new_id });
            }

            for (handle, point) in [
                (EntityPoint::Start(new_id), a),
                (EntityPoint::End(new_id), b),
            ] {
                if let Some(matched) = find_coincident_handle(sketch, new_id, point) {
                    to_add.push(Constraint::Coincident {
                        a: handle,
                        b: matched,
                    });
                }
            }
        }
        SketchEntity::Arc { .. } => {
            for handle in [
                EntityPoint::Start(new_id),
                EntityPoint::End(new_id),
                EntityPoint::Center(new_id),
            ] {
                let Some(point) = resolve_entity_point(handle, &entity) else {
                    continue;
                };
                if let Some(matched) = find_coincident_handle(sketch, new_id, point) {
                    to_add.push(Constraint::Coincident {
                        a: handle,
                        b: matched,
                    });
                }
            }
        }
        SketchEntity::Circle { center, .. } => {
            let handle = EntityPoint::Center(new_id);
            if let Some(matched) = find_coincident_handle(sketch, new_id, center) {
                to_add.push(Constraint::Coincident {
                    a: handle,
                    b: matched,
                });
            }
        }
        SketchEntity::Point { .. } | SketchEntity::Rectangle { .. } => {}
    }

    for c in to_add {
        sketch.add_constraint(c);
    }
}

/// First entity-point in `sketch` (other than ones on `skip_id`) whose
/// location matches `point` within epsilon.
fn find_coincident_handle(
    sketch: &Sketch,
    skip_id: SketchEntityId,
    point: glam::DVec2,
) -> Option<EntityPoint> {
    for (id, entity) in sketch.iter() {
        if id == skip_id {
            continue;
        }
        for handle in handles_for(id, entity) {
            if let Some(other) = resolve_entity_point(handle, entity) {
                if point.distance(other) < INFERENCE_EPSILON {
                    return Some(handle);
                }
            }
        }
    }
    None
}

fn handles_for(id: SketchEntityId, entity: &SketchEntity) -> Vec<EntityPoint> {
    match entity {
        SketchEntity::Line { .. } => vec![EntityPoint::Start(id), EntityPoint::End(id)],
        SketchEntity::Arc { .. } => vec![
            EntityPoint::Start(id),
            EntityPoint::End(id),
            EntityPoint::Center(id),
        ],
        SketchEntity::Circle { .. } => vec![EntityPoint::Center(id)],
        SketchEntity::Point { .. } | SketchEntity::Rectangle { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::*;
    use crate::Sketch;

    fn new_sketch() -> Sketch {
        Sketch::new("S", slotmap::KeyData::default().into())
    }

    #[test]
    fn horizontal_line_gets_horizontal_constraint() {
        let mut sketch = new_sketch();
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 2.0),
            b: dvec2(10.0, 2.0),
        });

        infer_constraints(&mut sketch, line);

        let constraints: Vec<_> = sketch.iter_constraints().map(|(_, c)| *c).collect();
        assert_eq!(constraints.len(), 1);
        assert!(matches!(
            constraints[0],
            Constraint::Horizontal { entity } if entity == line
        ));
    }

    #[test]
    fn vertical_line_gets_vertical_constraint() {
        let mut sketch = new_sketch();
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(3.0, 0.0),
            b: dvec2(3.0, 10.0),
        });

        infer_constraints(&mut sketch, line);

        assert!(sketch
            .iter_constraints()
            .any(|(_, c)| matches!(c, Constraint::Vertical { entity } if *entity == line)));
    }

    #[test]
    fn diagonal_line_gets_no_hv_constraint() {
        let mut sketch = new_sketch();
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 5.0),
        });

        infer_constraints(&mut sketch, line);

        assert!(!sketch.iter_constraints().any(|(_, c)| matches!(
            c,
            Constraint::Horizontal { .. } | Constraint::Vertical { .. }
        )));
    }

    #[test]
    fn endpoint_on_existing_endpoint_gets_coincident() {
        let mut sketch = new_sketch();
        let first = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        let second = sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 5.0),
        });

        infer_constraints(&mut sketch, second);

        let coincident: Vec<_> = sketch
            .iter_constraints()
            .filter_map(|(_, c)| match c {
                Constraint::Coincident { a, b } => Some((*a, *b)),
                _ => None,
            })
            .collect();
        assert_eq!(coincident.len(), 1);
        let (a, b) = coincident[0];
        assert!(
            (a == EntityPoint::Start(second) && b == EntityPoint::End(first))
                || (a == EntityPoint::End(first) && b == EntityPoint::Start(second))
        );
    }

    #[test]
    fn circle_center_on_existing_point_gets_coincident() {
        let mut sketch = new_sketch();
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(5.0, 5.0),
            b: dvec2(10.0, 5.0),
        });
        let circle = sketch.add(SketchEntity::Circle {
            center: dvec2(5.0, 5.0),
            radius: 2.0,
        });

        infer_constraints(&mut sketch, circle);

        assert!(sketch.iter_constraints().any(|(_, c)| matches!(
            c,
            Constraint::Coincident {
                a: EntityPoint::Center(ca),
                b: EntityPoint::Start(lb),
            } if *ca == circle && *lb == line
        )));
    }

    #[test]
    fn entity_removal_cascades_its_constraints() {
        let mut sketch = new_sketch();
        let first = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        let second = sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 5.0),
        });
        infer_constraints(&mut sketch, second);
        assert!(sketch.iter_constraints().count() >= 1);

        sketch.remove(first);

        assert!(!sketch
            .iter_constraints()
            .any(|(_, c)| c.referenced_entities().contains(&first)));
    }
}
