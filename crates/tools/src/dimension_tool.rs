//! Dimension tool: pick two points to create a persistent sketch distance
//! dimension. This is the first step before editable driven dimensions land.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

#[derive(Debug, Clone, Copy)]
enum DimensionState {
    Idle,
    Anchored { start: DVec2, cursor: DVec2 },
    Locked,
}

impl Default for DimensionState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct DimensionTool {
    state: DimensionState,
}

impl Tool for DimensionTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Dimension
    }

    fn on_pointer_move(&mut self, _ctx: &ToolContext<'_>, world_mm: DVec2) {
        if let DimensionState::Anchored { start, .. } = self.state {
            self.state = DimensionState::Anchored {
                start,
                cursor: world_mm,
            };
        }
    }

    fn on_pointer_click(
        &mut self,
        ctx: &ToolContext<'_>,
        world_mm: DVec2,
    ) -> Vec<AppCommand> {
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };

        let mut commands = Vec::new();
        self.state = match self.state {
            DimensionState::Idle => DimensionState::Anchored {
                start: world_mm,
                cursor: world_mm,
            },
            DimensionState::Anchored { start, .. } => {
                if (start - world_mm).length_squared() < f64::EPSILON {
                    DimensionState::Anchored {
                        start,
                        cursor: world_mm,
                    }
                } else {
                    commands.push(AppCommand::AddDistanceDimension {
                        sketch,
                        start,
                        end: world_mm,
                    });
                    DimensionState::Locked
                }
            }
            DimensionState::Locked => DimensionState::Anchored {
                start: world_mm,
                cursor: world_mm,
            },
        };
        commands
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.state = DimensionState::Idle;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.state = DimensionState::Idle;
    }

    fn preview(&self) -> ToolPreview {
        match self.state {
            DimensionState::Idle => ToolPreview::None,
            DimensionState::Anchored { start, cursor } => {
                ToolPreview::Measurement { start, end: cursor }
            }
            DimensionState::Locked => ToolPreview::None,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.state {
            DimensionState::Idle => {
                "Click first point to create a dimension. Shortcut: D. Right-click or Esc clears."
                    .to_string()
            }
            DimensionState::Anchored { .. } => {
                "Click second point to place persistent dimension. Right-click or Esc clears."
                    .to_string()
            }
            DimensionState::Locked => {
                "Dimension placed. Click anywhere to start a new one. Right-click or Esc clears."
                    .to_string()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::DimensionTool;
    use crate::tool::{Tool, ToolContext, ToolPreview};

    #[test]
    fn second_click_locks_measurement_preview() {
        let mut tool = DimensionTool::default();
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_move(&ctx, dvec2(3.0, 4.0));
        let commands = tool.on_pointer_click(&ctx, dvec2(3.0, 4.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
        assert!(matches!(
            commands.as_slice(),
            [AppCommand::AddDistanceDimension { start, end, .. }]
                if *start == dvec2(0.0, 0.0) && *end == dvec2(3.0, 4.0)
        ));
    }

    #[test]
    fn third_click_starts_new_measurement() {
        let mut tool = DimensionTool::default();
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_click(&ctx, dvec2(10.0, 0.0));
        tool.on_pointer_click(&ctx, dvec2(2.0, 2.0));

        assert!(matches!(
            tool.preview(),
            ToolPreview::Measurement { start, end }
                if start == dvec2(2.0, 2.0) && end == dvec2(2.0, 2.0)
        ));
    }

    #[test]
    fn right_click_clears_staged_dimension() {
        let mut tool = DimensionTool::default();
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        tool.on_pointer_move(&ctx, dvec2(3.0, 4.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(3.0, 4.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }
}
