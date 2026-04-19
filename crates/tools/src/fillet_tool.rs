//! Fillet tool: click a shared corner between two line segments, then move
//! the cursor to set radius and click again to replace the corner with
//! a tangent arc plus trimmed lines.

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::units::LengthMm;
use roncad_geometry::{find_line_fillet_candidate, LineFilletCandidate};

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview, ENTITY_PICK_RADIUS_PX};

#[derive(Default)]
pub struct FilletTool {
    staged: Option<LineFilletCandidate>,
    hovered: Option<LineFilletCandidate>,
    cursor: Option<DVec2>,
}

impl Tool for FilletTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Fillet
    }

    fn on_pointer_move(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(world_mm);
        if self.staged.is_some() {
            return;
        }

        let Some(sketch) = ctx.sketch else {
            self.hovered = None;
            return;
        };
        let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
        self.hovered = find_line_fillet_candidate(sketch, world_mm, tolerance_mm);
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        self.cursor = Some(world_mm);

        let Some(sketch_id) = ctx.active_sketch else {
            return Vec::new();
        };
        let Some(sketch) = ctx.sketch else {
            return Vec::new();
        };

        if let Some(candidate) = self.staged.as_ref() {
            let radius = candidate.radius_from_cursor(world_mm);
            if candidate.preview(radius).is_some() {
                let command = AppCommand::ApplyLineFillet {
                    sketch: sketch_id,
                    line_a: candidate.line_a,
                    line_b: candidate.line_b,
                    corner: candidate.corner,
                    radius: LengthMm::new(radius),
                };
                self.staged = None;
                self.hovered = None;
                self.cursor = None;
                return vec![command];
            }
            return Vec::new();
        }

        let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
        self.staged = self
            .hovered
            .clone()
            .or_else(|| find_line_fillet_candidate(sketch, world_mm, tolerance_mm));
        self.hovered = None;
        Vec::new()
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.staged = None;
        self.hovered = None;
        self.cursor = None;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.staged = None;
        self.hovered = None;
        self.cursor = None;
    }

    fn preview(&self) -> ToolPreview {
        if let Some(candidate) = self.staged.as_ref() {
            let Some(cursor) = self.cursor else {
                return ToolPreview::None;
            };
            let radius = candidate.radius_from_cursor(cursor);
            let Some(preview) = candidate.preview(radius) else {
                return ToolPreview::None;
            };

            return ToolPreview::Fillet {
                trim_a: preview.trim_a,
                trim_b: preview.trim_b,
                center: preview.center,
                radius: preview.radius,
                start_angle: preview.start_angle,
                sweep_angle: preview.sweep_angle,
            };
        }

        let Some(candidate) = self.hovered.as_ref() else {
            return ToolPreview::None;
        };
        let radius = hover_radius(candidate);
        let Some(preview) = candidate.preview(radius) else {
            return ToolPreview::None;
        };

        ToolPreview::FilletHover {
            corner: candidate.corner,
            trim_a: preview.trim_a,
            trim_b: preview.trim_b,
            center: preview.center,
            radius: preview.radius,
            start_angle: preview.start_angle,
            sweep_angle: preview.sweep_angle,
            max_radius: candidate.max_radius,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match (self.staged.as_ref(), self.hovered.as_ref(), self.cursor) {
            (None, Some(candidate), _) => format!(
                "Click to start fillet. Max radius {:.3} mm. Shortcut: F. Right-click or Esc cancels.",
                candidate.max_radius
            ),
            (None, None, _) => {
                "Click a corner shared by two lines. Shortcut: F. Right-click or Esc cancels."
                    .to_string()
            }
            (Some(candidate), _, Some(cursor)) => format!(
                "Move to set fillet radius {:.3} mm, then click to apply. Right-click or Esc cancels.",
                candidate.radius_from_cursor(cursor)
            ),
            (Some(_), _, None) => {
                "Move to set radius, then click to apply. Right-click or Esc cancels."
                    .to_string()
            }
        })
    }
}

fn hover_radius(candidate: &LineFilletCandidate) -> f64 {
    candidate
        .max_radius
        .min((candidate.max_radius * 0.25).max(0.75))
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::{Project, SketchEntity};

    use super::FilletTool;
    use crate::tool::{Modifiers, Tool, ToolContext, ToolPreview};

    #[test]
    fn corner_click_then_second_click_emits_line_fillet_command() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let line_a = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        let line_b = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(0.0, 10.0),
            });
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = FilletTool::default();

        assert!(tool.on_pointer_click(&ctx, dvec2(0.0, 0.0)).is_empty());
        tool.on_pointer_move(&ctx, dvec2(2.0, 2.0));
        let commands = tool.on_pointer_click(&ctx, dvec2(2.0, 2.0));

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            AppCommand::ApplyLineFillet {
                sketch: command_sketch,
                line_a: command_a,
                line_b: command_b,
                corner,
                radius,
            }
                if *command_sketch == sketch
                    && *corner == dvec2(0.0, 0.0)
                    && ((*command_a == line_a && *command_b == line_b)
                        || (*command_a == line_b && *command_b == line_a))
                    && (radius.as_f64() - 2.0).abs() < 1e-6
        ));
    }

    #[test]
    fn right_click_clears_staged_fillet_preview() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(0.0, 10.0),
            });
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = FilletTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_move(&ctx, dvec2(2.0, 2.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(2.0, 2.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }

    #[test]
    fn hovering_valid_corner_exposes_hover_indicator() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(0.0, 10.0),
            });
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = FilletTool::default();

        tool.on_pointer_move(&ctx, dvec2(0.1, 0.1));

        assert!(matches!(
            tool.preview(),
            ToolPreview::FilletHover {
                corner,
                max_radius,
                ..
            } if corner == dvec2(0.0, 0.0) && (max_radius - 10.0).abs() < 1e-6
        ));
    }
}
