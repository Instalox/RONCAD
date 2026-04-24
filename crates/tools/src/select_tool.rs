//! Select tool: click an entity to toggle it in the selection, Shift-click to
//! add, and Shift+Alt-click to subtract. Empty plain clicks clear selection.

use glam::DVec2;
use roncad_core::command::{AppCommand, SelectionEditMode};
use roncad_core::ids::{SketchEntityId, SketchId, ToolId};
use roncad_geometry::pick_entity;

use crate::tool::{ActiveToolKind, Modifiers, Tool, ToolContext, ENTITY_PICK_RADIUS_PX};

pub const SELECT_TOOL_ID: ToolId = ToolId::new("select");

#[derive(Default)]
pub struct SelectTool;

/// Shared commit logic for Select. Plain click preserves RonCad's existing
/// Shapr3D-style multi-selection by toggling the target without a modifier.
pub fn select_commands(
    target: Option<(SketchId, SketchEntityId)>,
    modifiers: Modifiers,
) -> Vec<AppCommand> {
    match (target, modifiers.shift, modifiers.alt) {
        (Some((sketch, entity)), true, true) => vec![AppCommand::SelectEntities {
            sketch,
            entities: vec![entity],
            mode: SelectionEditMode::Remove,
        }],
        (Some((sketch, entity)), true, false) => vec![AppCommand::SelectEntities {
            sketch,
            entities: vec![entity],
            mode: SelectionEditMode::Add,
        }],
        (Some((sketch, entity)), _, _) => vec![AppCommand::ToggleSelection { sketch, entity }],
        (None, false, false) => vec![AppCommand::ClearSelection],
        (None, _, _) => Vec::new(),
    }
}

impl Tool for SelectTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Select
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        let Some(sketch_id) = ctx.active_sketch else {
            return select_commands(None, ctx.modifiers);
        };
        let Some(sketch) = ctx.sketch else {
            return select_commands(None, ctx.modifiers);
        };

        let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
        let target = pick_entity(sketch, world_mm, tolerance_mm).map(|entity| (sketch_id, entity));
        select_commands(target, ctx.modifiers)
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
    use roncad_core::command::{AppCommand, SelectionEditMode};
    use roncad_geometry::{Project, SketchEntity};

    use super::SelectTool;
    use crate::tool::{Modifiers, Tool, ToolContext};

    #[test]
    fn plain_click_on_entity_toggles_selection() {
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
            AppCommand::ToggleSelection {
                sketch,
                entity: selected,
            } if sketch == sketch_id && selected == entity
        ));
    }

    #[test]
    fn shift_click_adds_to_selection() {
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
                shift: true,
                ..Modifiers::default()
            },
        };
        let mut tool = SelectTool;

        let commands = tool.on_pointer_click(&ctx, dvec2(14.0, 10.0));

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            AppCommand::SelectEntities {
                sketch,
                entities,
                mode: SelectionEditMode::Add,
            } if *sketch == sketch_id && entities == &vec![entity]
        ));
    }

    #[test]
    fn shift_alt_click_subtracts_from_selection() {
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
                shift: true,
                alt: true,
                ..Modifiers::default()
            },
        };
        let mut tool = SelectTool;

        let commands = tool.on_pointer_click(&ctx, dvec2(14.0, 10.0));

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            AppCommand::SelectEntities {
                sketch,
                entities,
                mode: SelectionEditMode::Remove,
            } if *sketch == sketch_id && entities == &vec![entity]
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
