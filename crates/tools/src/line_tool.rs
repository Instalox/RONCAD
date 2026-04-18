//! Line tool: click the first point, click the second to commit a line
//! segment. Chains consecutive segments (second click becomes next start).
//! Escape cancels.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{ActiveToolKind, Modifiers, Tool, ToolContext, ToolPreview};

#[derive(Default)]
pub struct LineTool {
    first_point: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for LineTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Line
    }

    fn on_pointer_move(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(match self.first_point {
            Some(start) => constrained_endpoint(start, world_mm, ctx.modifiers),
            None => world_mm,
        });
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
                self.cursor = Some(world_mm);
                Vec::new()
            }
            Some(start) => {
                let end = constrained_endpoint(start, world_mm, ctx.modifiers);
                if (start - end).length_squared() < f64::EPSILON {
                    return Vec::new();
                }
                let commands = vec![AppCommand::AddLine {
                    sketch,
                    a: start,
                    b: end,
                }];
                self.first_point = Some(end);
                self.cursor = Some(end);
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

    fn step_hint(&self) -> Option<String> {
        Some(match self.first_point {
            None => "Click first point. Shortcut: L. Esc cancels.".to_string(),
            Some(_) => "Click next point to place line. Hold Shift to lock axis. Esc cancels.".to_string(),
        })
    }
}

fn constrained_endpoint(start: DVec2, end: DVec2, modifiers: Modifiers) -> DVec2 {
    if !modifiers.shift {
        return end;
    }

    let delta = end - start;
    if delta.x.abs() >= delta.y.abs() {
        DVec2::new(end.x, start.y)
    } else {
        DVec2::new(start.x, end.y)
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::LineTool;
    use crate::tool::{Modifiers, Tool, ToolContext};

    #[test]
    fn shift_click_locks_line_to_dominant_axis() {
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
        let mut tool = LineTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        let commands = tool.on_pointer_click(&ctx, dvec2(10.0, 4.0));

        assert_eq!(
            commands,
            vec![AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            }]
        );
    }
}
