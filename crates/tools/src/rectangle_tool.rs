//! Rectangle tool: click one corner, click the opposite corner. The rectangle
//! is stored axis-aligned; rotated rects arrive once constraints land.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

#[derive(Default)]
pub struct RectangleTool {
    first_corner: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for RectangleTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Rectangle
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

        match self.first_corner.take() {
            None => {
                self.first_corner = Some(world_mm);
                self.cursor = Some(world_mm);
                Vec::new()
            }
            Some(first) => {
                if (first - world_mm).length_squared() < f64::EPSILON {
                    // Degenerate click — treat as re-pin.
                    self.first_corner = Some(world_mm);
                    return Vec::new();
                }
                vec![AppCommand::AddRectangle {
                    sketch,
                    corner_a: first,
                    corner_b: world_mm,
                }]
            }
        }
    }

    fn on_escape(&mut self) {
        self.first_corner = None;
    }

    fn preview(&self) -> ToolPreview {
        match (self.first_corner, self.cursor) {
            (Some(a), Some(b)) => ToolPreview::Rectangle {
                corner_a: a,
                corner_b: b,
            },
            _ => ToolPreview::None,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.first_corner {
            None => "Click first corner. Shortcut: R. Esc cancels.".to_string(),
            Some(_) => "Click opposite corner to place rectangle. Esc cancels.".to_string(),
        })
    }
}
