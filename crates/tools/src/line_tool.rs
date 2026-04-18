//! Line tool: click the first point, click the second to commit a line
//! segment. Chains consecutive segments (second click becomes next start).
//! Escape cancels.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

#[derive(Default)]
pub struct LineTool {
    first_point: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for LineTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Line
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

        match self.first_point {
            None => {
                self.first_point = Some(world_mm);
                Vec::new()
            }
            Some(start) => {
                if (start - world_mm).length_squared() < f64::EPSILON {
                    return Vec::new();
                }
                let commands = vec![AppCommand::AddLine {
                    sketch,
                    a: start,
                    b: world_mm,
                }];
                self.first_point = Some(world_mm);
                commands
            }
        }
    }

    fn on_escape(&mut self) {
        self.first_point = None;
    }

    fn preview(&self) -> ToolPreview {
        match (self.first_point, self.cursor) {
            (Some(start), Some(end)) => ToolPreview::Line { start, end },
            _ => ToolPreview::None,
        }
    }
}
