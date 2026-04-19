//! Snap engine. Stateless given configuration; invoked each frame by the
//! viewport with the raw cursor position plus the active sketch.
//!
//! Milestone 2 now supports grid snap, endpoint/center/midpoint snap, and
//! lightweight horizontal/vertical inference against nearby anchor points.

use glam::DVec2;
use roncad_geometry::{arc_end_point, arc_mid_point, arc_start_point, Sketch, SketchEntity};
use roncad_rendering::adaptive_grid_step_mm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Grid,
    Endpoint,
    Midpoint,
    Center,
    Horizontal,
    Vertical,
    Intersection,
}

impl SnapKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Endpoint => "Endpoint",
            Self::Midpoint => "Midpoint",
            Self::Center => "Center",
            Self::Horizontal => "H Align",
            Self::Vertical => "V Align",
            Self::Intersection => "H/V Align",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapReference {
    pub point: DVec2,
    pub kind: SnapKind,
    pub axis: Option<SnapAxis>,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    pub point: DVec2,
    pub kind: Option<SnapKind>,
    pub references: [Option<SnapReference>; 2],
}

impl SnapResult {
    pub fn raw(point: DVec2) -> Self {
        Self {
            point,
            kind: None,
            references: [None, None],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SnapEngine {
    pub grid_enabled: bool,
    pub endpoint_enabled: bool,
    /// Radius in pixels inside which direct and inferred snaps pull the cursor.
    pub pull_radius_px: f64,
    /// Minimum on-screen spacing for the adaptive grid (pixels).
    pub grid_min_spacing_px: f64,
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self {
            grid_enabled: true,
            endpoint_enabled: true,
            pull_radius_px: 10.0,
            grid_min_spacing_px: 8.0,
        }
    }
}

impl SnapEngine {
    pub fn snap(&self, raw: DVec2, sketch: Option<&Sketch>, pixels_per_mm: f64) -> SnapResult {
        let pull_mm = self.pull_radius_px / pixels_per_mm.max(f64::EPSILON);
        let grid_axes = self
            .grid_enabled
            .then(|| snapped_grid_axes(raw, pixels_per_mm, self.grid_min_spacing_px, pull_mm));

        if self.endpoint_enabled {
            if let Some(sketch) = sketch {
                if let Some(hit) = nearest_anchor(sketch, raw, pull_mm) {
                    return SnapResult {
                        point: hit.point,
                        kind: Some(hit.kind),
                        references: [
                            Some(SnapReference {
                                point: hit.point,
                                kind: hit.kind,
                                axis: None,
                            }),
                            None,
                        ],
                    };
                }

                let (x_hit, y_hit) = nearest_alignment(sketch, raw, pull_mm);
                if x_hit.is_some() || y_hit.is_some() {
                    return SnapResult {
                        point: DVec2::new(
                            x_hit.map_or_else(
                                || grid_axes.and_then(|axes| axes.x).unwrap_or(raw.x),
                                |hit| hit.point.x,
                            ),
                            y_hit.map_or_else(
                                || grid_axes.and_then(|axes| axes.y).unwrap_or(raw.y),
                                |hit| hit.point.y,
                            ),
                        ),
                        kind: Some(match (x_hit, y_hit) {
                            (Some(_), Some(_)) => SnapKind::Intersection,
                            (Some(_), None) => SnapKind::Vertical,
                            (None, Some(_)) => SnapKind::Horizontal,
                            (None, None) => unreachable!("checked above"),
                        }),
                        references: [
                            x_hit.map(|hit| SnapReference {
                                point: hit.point,
                                kind: hit.kind,
                                axis: Some(SnapAxis::Vertical),
                            }),
                            y_hit.map(|hit| SnapReference {
                                point: hit.point,
                                kind: hit.kind,
                                axis: Some(SnapAxis::Horizontal),
                            }),
                        ],
                    };
                }
            }
        }

        if self.grid_enabled {
            let axes = grid_axes.expect("computed when grid is enabled");
            if let Some(snapped) = axes.point_if_both() {
                return SnapResult {
                    point: snapped,
                    kind: Some(SnapKind::Grid),
                    references: [None, None],
                };
            }
        }

        SnapResult::raw(raw)
    }
}

#[derive(Debug, Clone, Copy)]
struct Anchor {
    point: DVec2,
    kind: SnapKind,
}

#[derive(Debug, Clone, Copy)]
struct AnchorHit {
    point: DVec2,
    kind: SnapKind,
    distance: f64,
}

#[derive(Debug, Clone, Copy)]
struct GridAxes {
    x: Option<f64>,
    y: Option<f64>,
}

impl GridAxes {
    fn point_if_both(self) -> Option<DVec2> {
        Some(DVec2::new(self.x?, self.y?))
    }
}

fn nearest_anchor(sketch: &Sketch, target: DVec2, tolerance_mm: f64) -> Option<AnchorHit> {
    let mut best: Option<AnchorHit> = None;
    for_each_anchor(sketch, |anchor| {
        let d = anchor.point.distance(target);
        if d <= tolerance_mm && best.as_ref().map_or(true, |b| d < b.distance) {
            best = Some(AnchorHit {
                point: anchor.point,
                kind: anchor.kind,
                distance: d,
            });
        }
    });
    best
}

fn nearest_alignment(
    sketch: &Sketch,
    target: DVec2,
    tolerance_mm: f64,
) -> (Option<AnchorHit>, Option<AnchorHit>) {
    let mut best_x: Option<AnchorHit> = None;
    let mut best_y: Option<AnchorHit> = None;

    for_each_anchor(sketch, |anchor| {
        let dx = (anchor.point.x - target.x).abs();
        if dx <= tolerance_mm && best_x.as_ref().map_or(true, |best| dx < best.distance) {
            best_x = Some(AnchorHit {
                point: anchor.point,
                kind: anchor.kind,
                distance: dx,
            });
        }

        let dy = (anchor.point.y - target.y).abs();
        if dy <= tolerance_mm && best_y.as_ref().map_or(true, |best| dy < best.distance) {
            best_y = Some(AnchorHit {
                point: anchor.point,
                kind: anchor.kind,
                distance: dy,
            });
        }
    });

    (best_x, best_y)
}

fn for_each_anchor(sketch: &Sketch, mut visit: impl FnMut(Anchor)) {
    for (_, entity) in sketch.iter() {
        match entity {
            SketchEntity::Point { p } => visit(Anchor {
                point: *p,
                kind: SnapKind::Endpoint,
            }),
            SketchEntity::Line { a, b } => {
                visit(Anchor {
                    point: *a,
                    kind: SnapKind::Endpoint,
                });
                visit(Anchor {
                    point: *b,
                    kind: SnapKind::Endpoint,
                });
                visit(Anchor {
                    point: (*a + *b) * 0.5,
                    kind: SnapKind::Midpoint,
                });
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                let min = corner_a.min(*corner_b);
                let max = corner_a.max(*corner_b);
                let corners = [
                    DVec2::new(min.x, min.y),
                    DVec2::new(max.x, min.y),
                    DVec2::new(max.x, max.y),
                    DVec2::new(min.x, max.y),
                ];
                for point in corners {
                    visit(Anchor {
                        point,
                        kind: SnapKind::Endpoint,
                    });
                }
                for point in [
                    DVec2::new((min.x + max.x) * 0.5, min.y),
                    DVec2::new(max.x, (min.y + max.y) * 0.5),
                    DVec2::new((min.x + max.x) * 0.5, max.y),
                    DVec2::new(min.x, (min.y + max.y) * 0.5),
                ] {
                    visit(Anchor {
                        point,
                        kind: SnapKind::Midpoint,
                    });
                }
            }
            SketchEntity::Circle { center, .. } => visit(Anchor {
                point: *center,
                kind: SnapKind::Center,
            }),
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => {
                visit(Anchor {
                    point: arc_start_point(*center, *radius, *start_angle),
                    kind: SnapKind::Endpoint,
                });
                visit(Anchor {
                    point: arc_end_point(*center, *radius, *start_angle, *sweep_angle),
                    kind: SnapKind::Endpoint,
                });
                visit(Anchor {
                    point: arc_mid_point(*center, *radius, *start_angle, *sweep_angle),
                    kind: SnapKind::Midpoint,
                });
                visit(Anchor {
                    point: *center,
                    kind: SnapKind::Center,
                });
            }
        }
    }
}

fn snapped_grid_axes(
    raw: DVec2,
    pixels_per_mm: f64,
    grid_min_spacing_px: f64,
    pull_mm: f64,
) -> GridAxes {
    let step = adaptive_grid_step_mm(pixels_per_mm, grid_min_spacing_px);
    let snapped_x = (raw.x / step).round() * step;
    let snapped_y = (raw.y / step).round() * step;

    GridAxes {
        x: ((snapped_x - raw.x).abs() <= pull_mm).then_some(snapped_x),
        y: ((snapped_y - raw.y).abs() <= pull_mm).then_some(snapped_y),
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_geometry::{Project, SketchEntity};

    use super::{SnapAxis, SnapEngine, SnapKind};

    #[test]
    fn snaps_to_line_midpoint() {
        let mut project = Project::new_untitled();
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        let engine = SnapEngine::default();

        let result = engine.snap(dvec2(5.2, 0.1), project.active_sketch(), 10.0);

        assert_eq!(result.kind, Some(SnapKind::Midpoint));
        assert_eq!(result.point, dvec2(5.0, 0.0));
        assert!(matches!(
            result.references[0],
            Some(reference)
                if reference.point == dvec2(5.0, 0.0)
                    && reference.kind == SnapKind::Midpoint
                    && reference.axis.is_none()
        ));
    }

    #[test]
    fn vertical_alignment_snaps_x_to_nearby_anchor() {
        let mut project = Project::new_untitled();
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Point {
                p: dvec2(12.0, 4.0),
            });
        let engine = SnapEngine::default();

        let result = engine.snap(dvec2(12.3, 20.3), project.active_sketch(), 10.0);

        assert_eq!(result.kind, Some(SnapKind::Vertical));
        assert_eq!(result.point, dvec2(12.0, 20.0));
        assert!(matches!(
            result.references[0],
            Some(reference)
                if reference.point == dvec2(12.0, 4.0)
                    && reference.kind == SnapKind::Endpoint
                    && reference.axis == Some(SnapAxis::Vertical)
        ));
        assert!(result.references[1].is_none());
    }

    #[test]
    fn intersection_alignment_uses_two_reference_points() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch_mut().expect("active sketch");
        sketch.add(SketchEntity::Point {
            p: dvec2(12.0, 4.0),
        });
        sketch.add(SketchEntity::Circle {
            center: dvec2(3.0, 20.0),
            radius: 5.0,
        });
        let engine = SnapEngine::default();

        let result = engine.snap(dvec2(12.2, 20.4), project.active_sketch(), 10.0);

        assert_eq!(result.kind, Some(SnapKind::Intersection));
        assert_eq!(result.point, dvec2(12.0, 20.0));
        assert!(matches!(
            result.references[0],
            Some(reference)
                if reference.point == dvec2(12.0, 4.0)
                    && reference.axis == Some(SnapAxis::Vertical)
        ));
        assert!(matches!(
            result.references[1],
            Some(reference)
                if reference.point == dvec2(3.0, 20.0)
                    && reference.kind == SnapKind::Center
                    && reference.axis == Some(SnapAxis::Horizontal)
        ));
    }

    #[test]
    fn horizontal_alignment_preserves_grid_on_free_axis() {
        let mut project = Project::new_untitled();
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Point {
                p: dvec2(4.0, 12.0),
            });
        let engine = SnapEngine::default();

        let result = engine.snap(dvec2(9.3, 11.8), project.active_sketch(), 10.0);

        assert_eq!(result.kind, Some(SnapKind::Horizontal));
        assert_eq!(result.point, dvec2(9.0, 12.0));
        assert!(matches!(
            result.references[1],
            Some(reference)
                if reference.point == dvec2(4.0, 12.0)
                    && reference.axis == Some(SnapAxis::Horizontal)
        ));
    }
}
