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

    fn on_escape(&mut self) {
        self.center = None;
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
}
