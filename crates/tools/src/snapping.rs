//! Snap engine. Stateless given configuration; invoked each frame by the
//! viewport with the raw cursor position plus the active sketch.
//!
//! Milestone 2 supports grid snap and endpoint/center snap. Midpoint,
//! intersection, and H/V inference slot in behind additional SnapKind
//! variants without disturbing this module's public surface.

use glam::DVec2;
use roncad_geometry::{Sketch, SketchEntity};
use roncad_rendering::adaptive_grid_step_mm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    Grid,
    Endpoint,
    Center,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    pub point: DVec2,
    pub kind: Option<SnapKind>,
}

impl SnapResult {
    pub fn raw(point: DVec2) -> Self {
        Self { point, kind: None }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SnapEngine {
    pub grid_enabled: bool,
    pub endpoint_enabled: bool,
    /// Radius in pixels inside which endpoint/center snaps pull the cursor.
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
    pub fn snap(
        &self,
        raw: DVec2,
        sketch: Option<&Sketch>,
        pixels_per_mm: f64,
    ) -> SnapResult {
        let pull_mm = self.pull_radius_px / pixels_per_mm.max(f64::EPSILON);

        if self.endpoint_enabled {
            if let Some(sketch) = sketch {
                if let Some(hit) = nearest_anchor(sketch, raw, pull_mm) {
                    return SnapResult {
                        point: hit.point,
                        kind: Some(hit.kind),
                    };
                }
            }
        }

        if self.grid_enabled {
            let step = adaptive_grid_step_mm(pixels_per_mm, self.grid_min_spacing_px);
            let snapped = DVec2::new(
                (raw.x / step).round() * step,
                (raw.y / step).round() * step,
            );
            if (snapped - raw).length() <= pull_mm {
                return SnapResult {
                    point: snapped,
                    kind: Some(SnapKind::Grid),
                };
            }
        }

        SnapResult::raw(raw)
    }
}

struct AnchorHit {
    point: DVec2,
    kind: SnapKind,
    distance: f64,
}

fn nearest_anchor(sketch: &Sketch, target: DVec2, tolerance_mm: f64) -> Option<AnchorHit> {
    let mut best: Option<AnchorHit> = None;
    let mut consider = |point: DVec2, kind: SnapKind| {
        let d = point.distance(target);
        if d <= tolerance_mm && best.as_ref().map_or(true, |b| d < b.distance) {
            best = Some(AnchorHit { point, kind, distance: d });
        }
    };

    for (_, entity) in sketch.iter() {
        match entity {
            SketchEntity::Point { p } => consider(*p, SnapKind::Endpoint),
            SketchEntity::Line { a, b } => {
                consider(*a, SnapKind::Endpoint);
                consider(*b, SnapKind::Endpoint);
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                consider(DVec2::new(corner_a.x, corner_a.y), SnapKind::Endpoint);
                consider(DVec2::new(corner_b.x, corner_a.y), SnapKind::Endpoint);
                consider(DVec2::new(corner_b.x, corner_b.y), SnapKind::Endpoint);
                consider(DVec2::new(corner_a.x, corner_b.y), SnapKind::Endpoint);
            }
            SketchEntity::Circle { center, .. } => {
                consider(*center, SnapKind::Center);
            }
        }
    }

    best
}
