//! Circle tool: click the center, click a point on the rim.

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::units::LengthMm;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

#[derive(Default)]
pub struct CircleTool {
    center: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for CircleTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Circle
    }

    fn on_pointer_move(&mut self, _ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(world_mm);
    }

    fn on_pointer_click(
        &mut self,
        ctx: &ToolContext<'_>,
        world_mm: DVec2,
    ) -> Vec<AppCommand> {
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };

        match self.center.take() {
            None => {
                self.center = Some(world_mm);
                self.cursor = Some(world_mm);
                Vec::new()
            }
            Some(center) => {
                let radius = center.distance(world_mm);
                if radius < f64::EPSILON {
                    self.center = Some(world_mm);
                    return Vec::new();
                }
                vec![AppCommand::AddCircle {
                    sketch,
                    center,
                    radius: LengthMm::new(radius),
                }]
            }
        }
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.center = None;
        self.cursor = None;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.center = None;
        self.cursor = None;
    }

    fn preview(&self) -> ToolPreview {
        match (self.center, self.cursor) {
            (Some(c), Some(p)) => ToolPreview::Circle {
                center: c,
                radius: c.distance(p),
            },
            _ => ToolPreview::None,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.center {
            None => "Click center point. Shortcut: C. Right-click or Esc cancels.".to_string(),
            Some(_) => "Click rim point to set radius. Right-click or Esc cancels.".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_geometry::Project;

    use super::CircleTool;
    use crate::tool::{Tool, ToolContext, ToolPreview};

    #[test]
    fn right_click_clears_staged_circle() {
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = CircleTool::default();

        tool.on_pointer_click(&ctx, dvec2(1.0, 1.0));
        tool.on_pointer_move(&ctx, dvec2(3.0, 1.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(3.0, 1.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }
}
