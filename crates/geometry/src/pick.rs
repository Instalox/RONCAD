//! Entity hit-testing. Returns the nearest sketch entity within a distance
//! tolerance, expressed in sketch-local mm (the caller converts pixels if
//! needed).

use glam::DVec2;
use roncad_core::ids::SketchEntityId;

use crate::{distance_to_arc, Sketch, SketchEntity};

/// Hit-test the sketch. Returns the id of the nearest entity whose visible
/// geometry lies within `tolerance_mm` of `world`, or None.
pub fn pick_entity(sketch: &Sketch, world: DVec2, tolerance_mm: f64) -> Option<SketchEntityId> {
    let mut best: Option<(SketchEntityId, f64)> = None;
    for (id, entity) in sketch.iter() {
        let d = distance_to_entity(entity, world);
        if d <= tolerance_mm && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((id, d));
        }
    }
    best.map(|(id, _)| id)
}

/// Hit-test the sketch and return every entity within `tolerance_mm`, sorted
/// nearest-first. Used by Select to expose the overlap stack so the user can
/// cycle through items under the cursor (Tab).
pub fn pick_entities_stack(
    sketch: &Sketch,
    world: DVec2,
    tolerance_mm: f64,
) -> Vec<SketchEntityId> {
    let mut hits: Vec<(SketchEntityId, f64)> = sketch
        .iter()
        .filter_map(|(id, entity)| {
            let d = distance_to_entity(entity, world);
            (d <= tolerance_mm).then_some((id, d))
        })
        .collect();
    hits.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    hits.into_iter().map(|(id, _)| id).collect()
}

pub fn distance_to_entity(entity: &SketchEntity, p: DVec2) -> f64 {
    match entity {
        SketchEntity::Point { p: q } => p.distance(*q),
        SketchEntity::Line { a, b } => distance_point_segment(p, *a, *b),
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let corners = [
                DVec2::new(corner_a.x, corner_a.y),
                DVec2::new(corner_b.x, corner_a.y),
                DVec2::new(corner_b.x, corner_b.y),
                DVec2::new(corner_a.x, corner_b.y),
            ];
            (0..4)
                .map(|i| distance_point_segment(p, corners[i], corners[(i + 1) % 4]))
                .fold(f64::INFINITY, f64::min)
        }
        SketchEntity::Circle { center, radius } => (p.distance(*center) - *radius).abs(),
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => distance_to_arc(p, *center, *radius, *start_angle, *sweep_angle),
    }
}

fn distance_point_segment(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= f64::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sketch;
    use glam::dvec2;

    #[test]
    fn pick_entities_stack_returns_closest_first() {
        let mut sketch = Sketch::new("t", slotmap::KeyData::default().into());
        let far = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 2.0),
            b: dvec2(10.0, 2.0),
        });
        let near = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.3),
            b: dvec2(10.0, 0.3),
        });
        let miss = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 50.0),
            b: dvec2(10.0, 50.0),
        });

        let stack = pick_entities_stack(&sketch, dvec2(5.0, 0.0), 5.0);

        assert_eq!(stack, vec![near, far]);
        assert!(!stack.contains(&miss));
    }

    #[test]
    fn pick_entities_stack_excludes_out_of_tolerance() {
        let mut sketch = Sketch::new("t", slotmap::KeyData::default().into());
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });

        let stack = pick_entities_stack(&sketch, dvec2(5.0, 0.5), 0.1);
        assert!(stack.is_empty());

        let stack = pick_entities_stack(&sketch, dvec2(5.0, 0.5), 1.0);
        assert_eq!(stack, vec![line]);
    }
}
