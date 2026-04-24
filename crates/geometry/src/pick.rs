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

/// Return entities selected by a world-space marquee rectangle.
///
/// When `crossing` is false, an entity must be fully contained. When true,
/// touching or crossing the rectangle is enough.
pub fn entities_in_selection_rect(
    sketch: &Sketch,
    min: DVec2,
    max: DVec2,
    crossing: bool,
) -> Vec<SketchEntityId> {
    sketch
        .iter()
        .filter_map(|(id, entity)| entity_matches_rect(entity, min, max, crossing).then_some(id))
        .collect()
}

pub fn entities_in_lasso(sketch: &Sketch, points: &[DVec2]) -> Vec<SketchEntityId> {
    if points.len() < 3 {
        return Vec::new();
    }
    sketch
        .iter()
        .filter_map(|(id, entity)| entity_matches_lasso(entity, points).then_some(id))
        .collect()
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

fn entity_matches_lasso(entity: &SketchEntity, lasso: &[DVec2]) -> bool {
    let (lasso_min, lasso_max) = points_bounds(lasso.iter().copied());
    let (entity_min, entity_max) = entity_bounds(entity);
    if !aabb_overlaps(entity_min, entity_max, lasso_min, lasso_max) {
        return false;
    }
    entity_sample_points(entity)
        .into_iter()
        .any(|point| point_in_polygon(point, lasso))
        || entity_segments(entity)
            .into_iter()
            .any(|(a, b)| polygon_edges(lasso).any(|(c, d)| segments_intersect(a, b, c, d)))
}

fn entity_matches_rect(entity: &SketchEntity, min: DVec2, max: DVec2, crossing: bool) -> bool {
    if crossing {
        return entity_crosses_rect(entity, min, max);
    }

    let (entity_min, entity_max) = entity_bounds(entity);
    point_in_rect(entity_min, min, max) && point_in_rect(entity_max, min, max)
}

fn entity_sample_points(entity: &SketchEntity) -> Vec<DVec2> {
    match entity {
        SketchEntity::Point { p } => vec![*p],
        SketchEntity::Line { a, b } => vec![*a, (*a + *b) * 0.5, *b],
        SketchEntity::Rectangle { corner_a, corner_b } => {
            rect_corners(*corner_a, *corner_b).to_vec()
        }
        SketchEntity::Circle { center, radius } => crate::arc_sample_points(
            *center,
            *radius,
            0.0,
            std::f64::consts::TAU,
            std::f64::consts::PI / 16.0,
        ),
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => crate::arc_sample_points(
            *center,
            *radius,
            *start_angle,
            *sweep_angle,
            std::f64::consts::PI / 24.0,
        ),
    }
}

fn entity_segments(entity: &SketchEntity) -> Vec<(DVec2, DVec2)> {
    match entity {
        SketchEntity::Point { .. } => Vec::new(),
        SketchEntity::Line { a, b } => vec![(*a, *b)],
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let corners = rect_corners(*corner_a, *corner_b);
            (0..4).map(|i| (corners[i], corners[(i + 1) % 4])).collect()
        }
        SketchEntity::Circle { center, radius } => {
            let points = crate::arc_sample_points(
                *center,
                *radius,
                0.0,
                std::f64::consts::TAU,
                std::f64::consts::PI / 24.0,
            );
            points.windows(2).map(|pair| (pair[0], pair[1])).collect()
        }
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            let points = crate::arc_sample_points(
                *center,
                *radius,
                *start_angle,
                *sweep_angle,
                std::f64::consts::PI / 24.0,
            );
            points.windows(2).map(|pair| (pair[0], pair[1])).collect()
        }
    }
}

fn point_in_polygon(point: DVec2, polygon: &[DVec2]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[j];
        if ((a.y > point.y) != (b.y > point.y))
            && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn polygon_edges(points: &[DVec2]) -> impl Iterator<Item = (DVec2, DVec2)> + '_ {
    points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
}

fn entity_crosses_rect(entity: &SketchEntity, min: DVec2, max: DVec2) -> bool {
    let (entity_min, entity_max) = entity_bounds(entity);
    if !aabb_overlaps(entity_min, entity_max, min, max) {
        return false;
    }
    if point_in_rect(entity_min, min, max) || point_in_rect(entity_max, min, max) {
        return true;
    }

    match entity {
        SketchEntity::Point { p } => point_in_rect(*p, min, max),
        SketchEntity::Line { a, b } => segment_crosses_rect(*a, *b, min, max),
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let corners = rect_corners(*corner_a, *corner_b);
            (0..4).any(|i| segment_crosses_rect(corners[i], corners[(i + 1) % 4], min, max))
                || marquee_contains_all_corners(min, max, &corners)
        }
        SketchEntity::Circle { center, radius } => {
            let closest = center.clamp(min, max);
            center.distance_squared(closest) <= radius * radius
        }
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            let samples = crate::arc_sample_points(
                *center,
                *radius,
                *start_angle,
                *sweep_angle,
                std::f64::consts::PI / 32.0,
            );
            samples
                .windows(2)
                .any(|pair| segment_crosses_rect(pair[0], pair[1], min, max))
                || samples.iter().any(|point| point_in_rect(*point, min, max))
        }
    }
}

fn entity_bounds(entity: &SketchEntity) -> (DVec2, DVec2) {
    match entity {
        SketchEntity::Point { p } => (*p, *p),
        SketchEntity::Line { a, b } => (a.min(*b), a.max(*b)),
        SketchEntity::Rectangle { corner_a, corner_b } => {
            (corner_a.min(*corner_b), corner_a.max(*corner_b))
        }
        SketchEntity::Circle { center, radius } => {
            let r = DVec2::splat(*radius);
            (*center - r, *center + r)
        }
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            let samples = crate::arc_sample_points(
                *center,
                *radius,
                *start_angle,
                *sweep_angle,
                std::f64::consts::PI / 32.0,
            );
            points_bounds(samples.into_iter())
        }
    }
}

fn points_bounds(points: impl IntoIterator<Item = DVec2>) -> (DVec2, DVec2) {
    let mut min = DVec2::splat(f64::INFINITY);
    let mut max = DVec2::splat(f64::NEG_INFINITY);
    for point in points {
        min = min.min(point);
        max = max.max(point);
    }
    (min, max)
}

fn rect_corners(a: DVec2, b: DVec2) -> [DVec2; 4] {
    [
        DVec2::new(a.x, a.y),
        DVec2::new(b.x, a.y),
        DVec2::new(b.x, b.y),
        DVec2::new(a.x, b.y),
    ]
}

fn marquee_contains_all_corners(min: DVec2, max: DVec2, corners: &[DVec2; 4]) -> bool {
    corners.iter().all(|point| point_in_rect(*point, min, max))
}

fn point_in_rect(p: DVec2, min: DVec2, max: DVec2) -> bool {
    p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y
}

fn aabb_overlaps(a_min: DVec2, a_max: DVec2, b_min: DVec2, b_max: DVec2) -> bool {
    a_min.x <= b_max.x && a_max.x >= b_min.x && a_min.y <= b_max.y && a_max.y >= b_min.y
}

fn segment_crosses_rect(a: DVec2, b: DVec2, min: DVec2, max: DVec2) -> bool {
    if point_in_rect(a, min, max) || point_in_rect(b, min, max) {
        return true;
    }
    let corners = rect_corners(min, max);
    (0..4).any(|i| segments_intersect(a, b, corners[i], corners[(i + 1) % 4]))
}

fn segments_intersect(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    const EPS: f64 = 1e-9;
    let ab = b - a;
    let cd = d - c;
    let denom = cross(ab, cd);
    let ac = c - a;
    if denom.abs() <= EPS {
        return cross(ac, ab).abs() <= EPS && aabb_overlaps(a.min(b), a.max(b), c.min(d), c.max(d));
    }
    let t = cross(ac, cd) / denom;
    let u = cross(ac, ab) / denom;
    (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u)
}

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
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

    #[test]
    fn selection_rect_distinguishes_contained_and_crossing() {
        let mut sketch = Sketch::new("t", slotmap::KeyData::default().into());
        let contained = sketch.add(SketchEntity::Line {
            a: dvec2(1.0, 1.0),
            b: dvec2(3.0, 1.0),
        });
        let crossing = sketch.add(SketchEntity::Line {
            a: dvec2(-1.0, 2.0),
            b: dvec2(3.0, 2.0),
        });
        let outside = sketch.add(SketchEntity::Circle {
            center: dvec2(10.0, 10.0),
            radius: 1.0,
        });

        let contained_only =
            entities_in_selection_rect(&sketch, dvec2(0.0, 0.0), dvec2(4.0, 4.0), false);
        assert_eq!(contained_only, vec![contained]);

        let crossing_hits =
            entities_in_selection_rect(&sketch, dvec2(0.0, 0.0), dvec2(4.0, 4.0), true);
        assert_eq!(crossing_hits, vec![contained, crossing]);
        assert!(!crossing_hits.contains(&outside));
    }

    #[test]
    fn lasso_selects_entities_inside_or_crossing_polygon() {
        let mut sketch = Sketch::new("t", slotmap::KeyData::default().into());
        let inside = sketch.add(SketchEntity::Point { p: dvec2(2.0, 2.0) });
        let crossing = sketch.add(SketchEntity::Line {
            a: dvec2(-1.0, 3.0),
            b: dvec2(3.0, 3.0),
        });
        let outside = sketch.add(SketchEntity::Line {
            a: dvec2(7.0, 7.0),
            b: dvec2(8.0, 8.0),
        });
        let lasso = vec![
            dvec2(0.0, 0.0),
            dvec2(4.0, 0.0),
            dvec2(4.0, 4.0),
            dvec2(0.0, 4.0),
        ];

        let hits = entities_in_lasso(&sketch, &lasso);

        assert!(hits.contains(&inside));
        assert!(hits.contains(&crossing));
        assert!(!hits.contains(&outside));
    }
}
