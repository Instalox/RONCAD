//! Select tool: click an entity to select it, Ctrl/Shift-click to toggle it,
//! click empty space to clear the current selection.

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::ids::ToolId;
use roncad_geometry::pick_entity;

use crate::tool::{ActiveToolKind, Tool, ToolContext, ENTITY_PICK_RADIUS_PX};

pub const SELECT_TOOL_ID: ToolId = ToolId::new("select");

#[derive(Default)]
pub struct SelectTool;

impl Tool for SelectTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Select
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        let additive = ctx.modifiers.ctrl || ctx.modifiers.shift;
        let Some(sketch_id) = ctx.active_sketch else {
            return if additive {
                Vec::new()
            } else {
                vec![AppCommand::ClearSelection]
            };
        };
        let Some(sketch) = ctx.sketch else {
            return if additive {
                Vec::new()
            } else {
                vec![AppCommand::ClearSelection]
            };
        };

        let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
        match pick_entity(sketch, world_mm, tolerance_mm) {
            Some(entity) if additive => vec![AppCommand::ToggleSelection {
                sketch: sketch_id,
                entity,
            }],
            Some(entity) => vec![AppCommand::SelectSingle {
                sketch: sketch_id,
                entity,
            }],
            None if additive => Vec::new(),
            None => vec![AppCommand::ClearSelection],
        }
    }
}

impl SelectTool {
    pub fn id(&self) -> ToolId {
        SELECT_TOOL_ID
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::{Project, SketchEntity};

    use super::SelectTool;
    use crate::tool::{Modifiers, Tool, ToolContext};

    #[test]
    fn click_on_entity_emits_single_select() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("default sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(20.0, 0.0),
            });
        let ctx = ToolContext {
            active_sketch: Some(sketch_id),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = SelectTool;

        let commands = tool.on_pointer_click(&ctx, dvec2(5.0, 0.1));

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            AppCommand::SelectSingle {
                sketch,
                entity: selected,
            } if sketch == sketch_id && selected == entity
        ));
    }

    #[test]
    fn ctrl_click_on_entity_emits_toggle() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("default sketch");
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Circle {
                    center: dvec2(10.0, 10.0),
                    radius: 4.0,
                });
        let ctx = ToolContext {
            active_sketch: Some(sketch_id),
            sketch: project.active_sketch(),
            pixels_per_mm: 12.0,
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        };
        let mut tool = SelectTool;

        let commands = tool.on_pointer_click(&ctx, dvec2(14.0, 10.0));

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            AppCommand::ToggleSelection {
                sketch,
                entity: selected,
            } if sketch == sketch_id && selected == entity
        ));
    }

    #[test]
    fn empty_click_clears_selection() {
        let project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch_id),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = SelectTool;

        let commands = tool.on_pointer_click(&ctx, dvec2(250.0, 250.0));

        assert_eq!(commands, vec![AppCommand::ClearSelection]);
    }
}
