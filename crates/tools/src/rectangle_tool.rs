//! Rectangle tool: click one corner, click the opposite corner. The rectangle
//! is stored axis-aligned; rotated rects arrive once constraints land.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{ActiveToolKind, Modifiers, Tool, ToolContext, ToolPreview};

#[derive(Default)]
pub struct RectangleTool {
    first_corner: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for RectangleTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Rectangle
    }

    fn on_pointer_move(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(match self.first_corner {
            Some(first) => constrained_corner(first, world_mm, ctx.modifiers),
            None => world_mm,
        });
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
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
                let corner_b = constrained_corner(first, world_mm, ctx.modifiers);
                if (first - corner_b).length_squared() < f64::EPSILON {
                    // Degenerate click — treat as re-pin.
                    self.first_corner = Some(world_mm);
                    return Vec::new();
                }
                vec![AppCommand::AddRectangle {
                    sketch,
                    corner_a: first,
                    corner_b,
                }]
            }
        }
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.first_corner = None;
        self.cursor = None;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.first_corner = None;
        self.cursor = None;
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
            None => {
                "Click first corner. Shortcut: R. Right-click or Esc cancels.".to_string()
            }
            Some(_) => {
                "Click opposite corner to place rectangle. Hold Shift to lock square. Right-click or Esc cancels."
                    .to_string()
            }
        })
    }
}

fn constrained_corner(corner_a: DVec2, corner_b: DVec2, modifiers: Modifiers) -> DVec2 {
    if !modifiers.shift {
        return corner_b;
    }

    let delta = corner_b - corner_a;
    let side = delta.x.abs().max(delta.y.abs());
    if side < f64::EPSILON {
        return corner_a;
    }

    DVec2::new(
        corner_a.x + side.copysign(delta.x),
        corner_a.y + side.copysign(delta.y),
    )
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::RectangleTool;
    use crate::tool::{Modifiers, Tool, ToolContext, ToolPreview};

    #[test]
    fn right_click_clears_staged_rectangle() {
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = RectangleTool::default();

        tool.on_pointer_click(&ctx, dvec2(1.0, 1.0));
        tool.on_pointer_move(&ctx, dvec2(4.0, 3.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(4.0, 3.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }

    #[test]
    fn shift_click_locks_rectangle_to_square() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        };
        let mut tool = RectangleTool::default();

        tool.on_pointer_click(&ctx, dvec2(2.0, 3.0));
        let commands = tool.on_pointer_click(&ctx, dvec2(8.0, 5.0));

        assert_eq!(
            commands,
            vec![AppCommand::AddRectangle {
                sketch,
                corner_a: dvec2(2.0, 3.0),
                corner_b: dvec2(8.0, 9.0),
            }]
        );
    }
}
