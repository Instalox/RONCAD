//! Arc tool: click center, click the start point, then click the end point.
//! The end point is projected back onto the locked radius so the preview
//! remains a clean circular arc as the cursor moves.

use std::f64::consts::{PI, TAU};

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::units::LengthMm;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

const ARC_EPSILON: f64 = 1e-6;

#[derive(Debug, Clone, Copy)]
enum ArcState {
    Idle,
    CenterLocked {
        center: DVec2,
        cursor: DVec2,
    },
    StartLocked {
        center: DVec2,
        radius: f64,
        start_angle: f64,
        cursor: DVec2,
    },
}

impl Default for ArcState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct ArcTool {
    state: ArcState,
}

impl Tool for ArcTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Arc
    }

    fn on_pointer_move(&mut self, _ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.state = match self.state {
            ArcState::Idle => ArcState::Idle,
            ArcState::CenterLocked { center, .. } => ArcState::CenterLocked {
                center,
                cursor: world_mm,
            },
            ArcState::StartLocked {
                center,
                radius,
                start_angle,
                ..
            } => ArcState::StartLocked {
                center,
                radius,
                start_angle,
                cursor: project_onto_radius(center, radius, start_angle, world_mm),
            },
        };
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };

        match self.state {
            ArcState::Idle => {
                self.state = ArcState::CenterLocked {
                    center: world_mm,
                    cursor: world_mm,
                };
                Vec::new()
            }
            ArcState::CenterLocked { center, .. } => {
                let radius = center.distance(world_mm);
                if radius <= ARC_EPSILON {
                    self.state = ArcState::CenterLocked {
                        center,
                        cursor: world_mm,
                    };
                    return Vec::new();
                }

                let start = project_onto_radius(center, radius, 0.0, world_mm);
                self.state = ArcState::StartLocked {
                    center,
                    radius,
                    start_angle: angle_of(center, start),
                    cursor: start,
                };
                Vec::new()
            }
            ArcState::StartLocked {
                center,
                radius,
                start_angle,
                ..
            } => {
                let end = project_onto_radius(center, radius, start_angle, world_mm);
                let sweep_angle = shortest_signed_angle(start_angle, angle_of(center, end));
                if sweep_angle.abs() <= ARC_EPSILON {
                    self.state = ArcState::StartLocked {
                        center,
                        radius,
                        start_angle,
                        cursor: end,
                    };
                    return Vec::new();
                }

                self.state = ArcState::Idle;
                vec![AppCommand::AddArc {
                    sketch,
                    center,
                    radius: LengthMm::new(radius),
                    start_angle,
                    sweep_angle,
                }]
            }
        }
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.state = ArcState::Idle;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.state = ArcState::Idle;
    }

    fn preview(&self) -> ToolPreview {
        match self.state {
            ArcState::Idle => ToolPreview::None,
            ArcState::CenterLocked { center, cursor } => ToolPreview::ArcRadius {
                center,
                radius: center.distance(cursor),
                rim: cursor,
            },
            ArcState::StartLocked {
                center,
                radius,
                start_angle,
                cursor,
            } => ToolPreview::Arc {
                center,
                radius,
                start_angle,
                sweep_angle: shortest_signed_angle(start_angle, angle_of(center, cursor)),
            },
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.state {
            ArcState::Idle => {
                "Click arc center. Shortcut: A. Right-click or Esc cancels.".to_string()
            }
            ArcState::CenterLocked { .. } => {
                "Click arc start point to lock radius. Right-click or Esc cancels.".to_string()
            }
            ArcState::StartLocked { radius, .. } => {
                format!(
                    "Click end point to place minor arc. R {:.3} mm. Right-click or Esc cancels.",
                    radius
                )
            }
        })
    }
}

fn project_onto_radius(center: DVec2, radius: f64, fallback_angle: f64, point: DVec2) -> DVec2 {
    let delta = point - center;
    if delta.length_squared() <= ARC_EPSILON * ARC_EPSILON {
        center + DVec2::new(fallback_angle.cos() * radius, fallback_angle.sin() * radius)
    } else {
        center + delta.normalize() * radius
    }
}

fn angle_of(center: DVec2, point: DVec2) -> f64 {
    let delta = point - center;
    delta.y.atan2(delta.x)
}

fn shortest_signed_angle(start_angle: f64, end_angle: f64) -> f64 {
    let delta = (end_angle - start_angle).rem_euclid(TAU);
    if delta > PI {
        delta - TAU
    } else {
        delta
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::ArcTool;
    use crate::tool::{Tool, ToolContext, ToolPreview};

    #[test]
    fn third_click_emits_arc_command() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = ArcTool::default();

        assert!(tool.on_pointer_click(&ctx, dvec2(0.0, 0.0)).is_empty());
        assert!(tool.on_pointer_click(&ctx, dvec2(10.0, 0.0)).is_empty());
        let commands = tool.on_pointer_click(&ctx, dvec2(0.0, 10.0));

        assert!(matches!(
            commands.as_slice(),
            [AppCommand::AddArc {
                sketch: command_sketch,
                center,
                radius,
                start_angle,
                sweep_angle,
            }]
                if *command_sketch == sketch
                    && *center == dvec2(0.0, 0.0)
                    && (radius.as_f64() - 10.0).abs() < 1e-6
                    && (*start_angle - 0.0).abs() < 1e-6
                    && (*sweep_angle - FRAC_PI_2).abs() < 1e-6
        ));
    }

    #[test]
    fn preview_projects_cursor_back_to_locked_radius() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = ArcTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_click(&ctx, dvec2(10.0, 0.0));
        tool.on_pointer_move(&ctx, dvec2(-8.0, 2.0));

        assert!(matches!(
            tool.preview(),
            ToolPreview::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            }
                if center == dvec2(0.0, 0.0)
                    && (radius - 10.0).abs() < 1e-6
                    && (start_angle - 0.0).abs() < 1e-6
                    && sweep_angle > 2.8
                    && sweep_angle < PI
        ));
    }

    #[test]
    fn right_click_clears_arc_state() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = ArcTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_click(&ctx, dvec2(10.0, 0.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(0.0, 10.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }
}
