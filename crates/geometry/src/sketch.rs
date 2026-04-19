//! A 2D sketch bound to a workplane. Owns its entities and persistent
//! dimensions; constraints and solving land later.

use std::collections::HashMap;

use glam::DVec2;
use roncad_core::ids::{SketchDimensionId, SketchEntityId, WorkplaneId};
use slotmap::SlotMap;

use crate::arc_sample_points;
use crate::sketch_dimension::SketchDimension;
use crate::sketch_entity::SketchEntity;

const SPLIT_EPSILON: f64 = 1e-6;

#[derive(Debug, Default, Clone)]
pub struct LineInsertResult {
    pub inserted: Vec<SketchEntityId>,
    pub replaced: Vec<LineReplacement>,
}

#[derive(Debug, Default, Clone)]
pub struct LineReplacement {
    pub original: SketchEntityId,
    pub segments: Vec<SketchEntityId>,
}

#[derive(Debug, Clone)]
pub struct Sketch {
    pub name: String,
    pub workplane: WorkplaneId,
    pub entities: SlotMap<SketchEntityId, SketchEntity>,
    pub dimensions: SlotMap<SketchDimensionId, SketchDimension>,
}

impl Sketch {
    pub fn new(name: impl Into<String>, workplane: WorkplaneId) -> Self {
        Self {
            name: name.into(),
            workplane,
            entities: SlotMap::with_key(),
            dimensions: SlotMap::with_key(),
        }
    }

    pub fn add(&mut self, entity: SketchEntity) -> SketchEntityId {
        self.entities.insert(entity)
    }

    pub fn add_line_with_splits(&mut self, a: DVec2, b: DVec2) -> LineInsertResult {
        if a.distance_squared(b) <= SPLIT_EPSILON * SPLIT_EPSILON {
            return LineInsertResult::default();
        }

        let existing_lines: Vec<_> = self
            .iter()
            .filter_map(|(id, entity)| match entity {
                SketchEntity::Line { a, b } => Some((id, *a, *b)),
                _ => None,
            })
            .collect();

        let mut new_line_splits = Vec::new();
        let mut existing_splits: HashMap<SketchEntityId, Vec<DVec2>> = HashMap::new();

        for (id, c, d) in &existing_lines {
            let Some(hit) = segment_intersection(a, b, *c, *d) else {
                continue;
            };

            if is_strictly_inside_param(hit.t_ab) {
                new_line_splits.push(hit.point);
            }
            if is_strictly_inside_param(hit.t_cd) {
                existing_splits.entry(*id).or_default().push(hit.point);
            }
        }

        let mut replaced = Vec::new();
        for (id, c, d) in existing_lines {
            let Some(points) = existing_splits.remove(&id) else {
                continue;
            };
            self.remove(id);
            let segments = split_segment(c, d, &points)
                .into_iter()
                .map(|(a, b)| self.add(SketchEntity::Line { a, b }))
                .collect();
            replaced.push(LineReplacement {
                original: id,
                segments,
            });
        }

        let inserted = split_segment(a, b, &new_line_splits)
            .into_iter()
            .map(|(a, b)| self.add(SketchEntity::Line { a, b }))
            .collect();

        LineInsertResult { inserted, replaced }
    }

    pub fn remove(&mut self, id: SketchEntityId) -> Option<SketchEntity> {
        self.entities.remove(id)
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

#[derive(Debug, Clone, Copy)]
struct SegmentIntersection {
    point: DVec2,
    t_ab: f64,
    t_cd: f64,
}

fn segment_intersection(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Option<SegmentIntersection> {
    let r = b - a;
    let s = d - c;
    let denom = cross(r, s);
    if denom.abs() <= SPLIT_EPSILON {
        return None;
    }

    let q = c - a;
    let t_ab = cross(q, s) / denom;
    let t_cd = cross(q, r) / denom;
    if !(-SPLIT_EPSILON..=1.0 + SPLIT_EPSILON).contains(&t_ab)
        || !(-SPLIT_EPSILON..=1.0 + SPLIT_EPSILON).contains(&t_cd)
    {
        return None;
    }

    Some(SegmentIntersection {
        point: a + r * t_ab.clamp(0.0, 1.0),
        t_ab,
        t_cd,
    })
}

fn split_segment(a: DVec2, b: DVec2, split_points: &[DVec2]) -> Vec<(DVec2, DVec2)> {
    let mut points = Vec::with_capacity(split_points.len() + 2);
    points.push(a);
    points.extend(
        split_points
            .iter()
            .copied()
            .filter(|point| is_strictly_inside_param(segment_parameter(a, b, *point))),
    );
    points.push(b);

    points.sort_by(|lhs, rhs| {
        segment_parameter(a, b, *lhs).total_cmp(&segment_parameter(a, b, *rhs))
    });
    points.dedup_by(|lhs, rhs| lhs.distance_squared(*rhs) <= SPLIT_EPSILON * SPLIT_EPSILON);

    points
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];
            (start.distance_squared(end) > SPLIT_EPSILON * SPLIT_EPSILON).then_some((start, end))
        })
        .collect()
}

fn segment_parameter(a: DVec2, b: DVec2, point: DVec2) -> f64 {
    let delta = b - a;
    if delta.x.abs() >= delta.y.abs() && delta.x.abs() > SPLIT_EPSILON {
        (point.x - a.x) / delta.x
    } else if delta.y.abs() > SPLIT_EPSILON {
        (point.y - a.y) / delta.y
    } else {
        0.0
    }
}

fn is_strictly_inside_param(t: f64) -> bool {
    t > SPLIT_EPSILON && t < 1.0 - SPLIT_EPSILON
}

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

#[cfg(test)]
mod tests {
    use glam::{dvec2, DVec2};

    use super::Sketch;
    use crate::SketchEntity;

    #[test]
    fn crossing_lines_split_into_four_segments() {
        let mut sketch = Sketch::new("Sketch", slotmap::KeyData::default().into());
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 10.0),
        });

        let result = sketch.add_line_with_splits(dvec2(0.0, 10.0), dvec2(10.0, 0.0));

        assert_eq!(result.inserted.len(), 2);
        assert_eq!(result.replaced.len(), 1);
        let lines: Vec<_> = sketch
            .iter()
            .filter_map(|(_, entity)| match entity {
                SketchEntity::Line { a, b } => Some((*a, *b)),
                _ => None,
            })
            .collect();

        assert_eq!(lines.len(), 4);
        assert!(contains_line(&lines, dvec2(0.0, 0.0), dvec2(5.0, 5.0)));
        assert!(contains_line(&lines, dvec2(5.0, 5.0), dvec2(10.0, 10.0)));
        assert!(contains_line(&lines, dvec2(0.0, 10.0), dvec2(5.0, 5.0)));
        assert!(contains_line(&lines, dvec2(5.0, 5.0), dvec2(10.0, 0.0)));
    }

    #[test]
    fn t_junction_splits_only_existing_line() {
        let mut sketch = Sketch::new("Sketch", slotmap::KeyData::default().into());
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });

        let result = sketch.add_line_with_splits(dvec2(5.0, -4.0), dvec2(5.0, 0.0));

        assert_eq!(result.inserted.len(), 1);
        assert_eq!(result.replaced.len(), 1);
        let lines: Vec<_> = sketch
            .iter()
            .filter_map(|(_, entity)| match entity {
                SketchEntity::Line { a, b } => Some((*a, *b)),
                _ => None,
            })
            .collect();

        assert_eq!(lines.len(), 3);
        assert!(contains_line(&lines, dvec2(0.0, 0.0), dvec2(5.0, 0.0)));
        assert!(contains_line(&lines, dvec2(5.0, 0.0), dvec2(10.0, 0.0)));
        assert!(contains_line(&lines, dvec2(5.0, -4.0), dvec2(5.0, 0.0)));
    }

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

    fn contains_line(lines: &[(DVec2, DVec2)], a: DVec2, b: DVec2) -> bool {
        lines.iter().any(|(line_a, line_b)| {
            (*line_a == a && *line_b == b) || (*line_a == b && *line_b == a)
        })
    }
}
