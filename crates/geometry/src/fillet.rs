//! Sketch fillet helpers for connected line segments.

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

use glam::DVec2;
use roncad_core::ids::SketchEntityId;

use crate::{Sketch, SketchEntity};

const FILLET_EPSILON: f64 = 1e-6;
const FILLET_QUANTIZE_SCALE: f64 = 1_000_000.0;

#[derive(Debug, Clone)]
pub struct LineFilletCandidate {
    pub line_a: SketchEntityId,
    pub line_b: SketchEntityId,
    pub corner: DVec2,
    pub other_a: DVec2,
    pub other_b: DVec2,
    pub max_radius: f64,
    pub bisector: DVec2,
    interior_angle: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct LineFilletPreview {
    pub trim_a: (DVec2, DVec2),
    pub trim_b: (DVec2, DVec2),
    pub center: DVec2,
    pub radius: f64,
    pub start_angle: f64,
    pub sweep_angle: f64,
}

#[derive(Debug, Default, Clone)]
pub struct LineFilletApplyResult {
    pub removed: [Option<SketchEntityId>; 2],
    pub inserted_lines: Vec<SketchEntityId>,
    pub inserted_arc: Option<SketchEntityId>,
}

impl LineFilletCandidate {
    pub fn radius_from_cursor(&self, cursor: DVec2) -> f64 {
        let distance_along_bisector = (cursor - self.corner).dot(self.bisector).max(0.0);
        let radius = distance_along_bisector * (self.interior_angle * 0.5).sin();
        radius.clamp(0.0, self.max_radius)
    }

    pub fn preview(&self, radius: f64) -> Option<LineFilletPreview> {
        let radius = radius.clamp(0.0, self.max_radius);
        if radius <= FILLET_EPSILON {
            return None;
        }

        let dir_a = (self.other_a - self.corner).normalize();
        let dir_b = (self.other_b - self.corner).normalize();
        let trim_distance = radius / (self.interior_angle * 0.5).tan();
        let tangent_a = self.corner + dir_a * trim_distance;
        let tangent_b = self.corner + dir_b * trim_distance;
        let center_distance = radius / (self.interior_angle * 0.5).sin();
        let center = self.corner + self.bisector * center_distance;

        let start_angle = (tangent_a - center).y.atan2((tangent_a - center).x);
        let end_angle = (tangent_b - center).y.atan2((tangent_b - center).x);
        let target_sweep = PI - self.interior_angle;
        let ccw = (end_angle - start_angle).rem_euclid(TAU);
        let cw = ccw - TAU;
        let sweep_angle = if (ccw.abs() - target_sweep).abs() <= (cw.abs() - target_sweep).abs() {
            ccw
        } else {
            cw
        };

        Some(LineFilletPreview {
            trim_a: (self.other_a, tangent_a),
            trim_b: (self.other_b, tangent_b),
            center,
            radius,
            start_angle,
            sweep_angle,
        })
    }
}

pub fn find_line_fillet_candidate(
    sketch: &Sketch,
    point: DVec2,
    tolerance_mm: f64,
) -> Option<LineFilletCandidate> {
    let mut best: Option<(LineFilletCandidate, f64)> = None;
    let mut seen = HashSet::new();

    for (line_id, entity) in sketch.iter() {
        let SketchEntity::Line { a, b } = entity else {
            continue;
        };

        for corner in [*a, *b] {
            if corner.distance(point) > tolerance_mm {
                continue;
            }
            if !seen.insert(QuantizedPoint::from_point(corner)) {
                continue;
            }

            let mut incident = Vec::new();
            for (other_id, other_entity) in sketch.iter() {
                let SketchEntity::Line { a, b } = other_entity else {
                    continue;
                };
                if same_point(*a, corner) {
                    incident.push((other_id, *b));
                } else if same_point(*b, corner) {
                    incident.push((other_id, *a));
                }
            }

            if incident.len() != 2 {
                continue;
            }
            let [(line_a, other_a), (line_b, other_b)] = [incident[0], incident[1]];
            let Some(candidate) = build_candidate(line_a, other_a, line_b, other_b, corner) else {
                continue;
            };

            let distance = corner.distance(point);
            if best
                .as_ref()
                .map_or(true, |(_, best_distance)| distance < *best_distance)
            {
                best = Some((candidate, distance));
            }
        }
        let _ = line_id; // quiet unused binding in pattern-driven loop
    }

    best.map(|(candidate, _)| candidate)
}

pub fn fillet_candidate_for_lines(
    sketch: &Sketch,
    line_a: SketchEntityId,
    line_b: SketchEntityId,
    corner: DVec2,
) -> Option<LineFilletCandidate> {
    let line_a_entity = sketch.entities.get(line_a)?;
    let line_b_entity = sketch.entities.get(line_b)?;
    let SketchEntity::Line { a: a0, b: a1 } = line_a_entity else {
        return None;
    };
    let SketchEntity::Line { a: b0, b: b1 } = line_b_entity else {
        return None;
    };

    let other_a = if same_point(*a0, corner) {
        *a1
    } else if same_point(*a1, corner) {
        *a0
    } else {
        return None;
    };
    let other_b = if same_point(*b0, corner) {
        *b1
    } else if same_point(*b1, corner) {
        *b0
    } else {
        return None;
    };

    build_candidate(line_a, other_a, line_b, other_b, corner)
}

pub fn apply_line_fillet(
    sketch: &mut Sketch,
    line_a: SketchEntityId,
    line_b: SketchEntityId,
    corner: DVec2,
    radius: f64,
) -> Option<LineFilletApplyResult> {
    let candidate = fillet_candidate_for_lines(sketch, line_a, line_b, corner)?;
    let preview = candidate.preview(radius)?;

    sketch.remove(line_a)?;
    sketch.remove(line_b)?;

    let mut inserted_lines = Vec::new();
    for (start, end) in [preview.trim_a, preview.trim_b] {
        if start.distance_squared(end) > FILLET_EPSILON * FILLET_EPSILON {
            inserted_lines.push(sketch.add(SketchEntity::Line { a: start, b: end }));
        }
    }

    let inserted_arc = sketch.add(SketchEntity::Arc {
        center: preview.center,
        radius: preview.radius,
        start_angle: preview.start_angle,
        sweep_angle: preview.sweep_angle,
    });

    Some(LineFilletApplyResult {
        removed: [Some(line_a), Some(line_b)],
        inserted_lines,
        inserted_arc: Some(inserted_arc),
    })
}

fn build_candidate(
    line_a: SketchEntityId,
    other_a: DVec2,
    line_b: SketchEntityId,
    other_b: DVec2,
    corner: DVec2,
) -> Option<LineFilletCandidate> {
    let vec_a = other_a - corner;
    let vec_b = other_b - corner;
    let len_a = vec_a.length();
    let len_b = vec_b.length();
    if len_a <= FILLET_EPSILON || len_b <= FILLET_EPSILON {
        return None;
    }

    let dir_a = vec_a / len_a;
    let dir_b = vec_b / len_b;
    let dot = dir_a.dot(dir_b).clamp(-1.0, 1.0);
    let interior_angle = dot.acos();
    let bisector_raw = dir_a + dir_b;
    if interior_angle <= FILLET_EPSILON
        || interior_angle >= PI - FILLET_EPSILON
        || bisector_raw.length_squared() <= FILLET_EPSILON * FILLET_EPSILON
    {
        return None;
    }

    let max_radius = len_a.min(len_b) * (interior_angle * 0.5).tan();
    if max_radius <= FILLET_EPSILON {
        return None;
    }

    Some(LineFilletCandidate {
        line_a,
        line_b,
        corner,
        other_a,
        other_b,
        max_radius,
        bisector: bisector_raw.normalize(),
        interior_angle,
    })
}

fn same_point(a: DVec2, b: DVec2) -> bool {
    a.distance_squared(b) <= FILLET_EPSILON * FILLET_EPSILON
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct QuantizedPoint(i64, i64);

impl QuantizedPoint {
    fn from_point(point: DVec2) -> Self {
        Self(
            (point.x * FILLET_QUANTIZE_SCALE).round() as i64,
            (point.y * FILLET_QUANTIZE_SCALE).round() as i64,
        )
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::ids::WorkplaneId;

    use super::{apply_line_fillet, find_line_fillet_candidate};
    use crate::{Sketch, SketchEntity};

    #[test]
    fn finds_shared_corner_candidate_for_two_lines() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(0.0, 10.0),
        });

        let candidate =
            find_line_fillet_candidate(&sketch, dvec2(0.2, 0.1), 0.5).expect("candidate");

        assert_eq!(candidate.corner, dvec2(0.0, 0.0));
        assert!((candidate.max_radius - 10.0).abs() < 1e-6);
    }

    #[test]
    fn applies_fillet_and_replaces_corner_with_arc() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        let line_a = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        let line_b = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(0.0, 10.0),
        });

        let result = apply_line_fillet(&mut sketch, line_a, line_b, dvec2(0.0, 0.0), 2.0)
            .expect("fillet result");

        assert_eq!(result.inserted_lines.len(), 2);
        assert!(result.inserted_arc.is_some());
        let entities: Vec<_> = sketch.iter().map(|(_, entity)| entity.clone()).collect();
        assert_eq!(entities.len(), 3);
        assert!(entities.iter().any(|entity| {
            matches!(
                entity,
                SketchEntity::Arc {
                    center,
                    radius,
                    sweep_angle,
                    ..
                }
                    if (*center - dvec2(2.0, 2.0)).length() < 1e-6
                        && (*radius - 2.0).abs() < 1e-6
                        && (sweep_angle.abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-6
            )
        }));
    }
}
