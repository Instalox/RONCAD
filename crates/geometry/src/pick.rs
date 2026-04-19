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
