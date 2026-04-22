//! Routes the active ActiveToolKind to a concrete Tool implementation.
//! The shell owns a ToolManager and the viewport drives it with events.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::arc_tool::ArcTool;
use crate::circle_tool::CircleTool;
use crate::dimension_tool::DimensionTool;
use crate::dynamic_input::{DynamicFieldView, DynamicInputState};
use crate::fillet_tool::FilletTool;
use crate::line_tool::LineTool;
use crate::rectangle_tool::RectangleTool;
use crate::select_tool::SelectTool;
use crate::tool::{ActiveToolKind, DynamicField, Tool, ToolContext, ToolPreview};

pub struct ToolManager {
    active_kind: ActiveToolKind,
    tool: Box<dyn Tool>,
    dynamic: DynamicInputState,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            active_kind: ActiveToolKind::Select,
            tool: Box::new(SelectTool::default()),
            dynamic: DynamicInputState::default(),
        }
    }

    pub fn active_kind(&self) -> ActiveToolKind {
        self.active_kind
    }

    pub fn set_active(&mut self, kind: ActiveToolKind) {
        if kind == self.active_kind {
            return;
        }
        self.tool.on_escape();
        self.active_kind = kind;
        self.tool = make_tool(kind);
        self.dynamic.clear();
    }

    pub fn on_pointer_move(&mut self, ctx: &ToolContext, world_mm: DVec2) {
        self.tool.on_pointer_move(ctx, world_mm);
    }

    pub fn on_pointer_click(&mut self, ctx: &ToolContext, world_mm: DVec2) -> Vec<AppCommand> {
        if !self.dynamic_fields().is_empty() && self.dynamic.has_any_text() {
            return self
                .dynamic
                .commit_if_valid(self.tool.as_mut(), ctx, world_mm);
        }

        let commands = self.tool.on_pointer_click(ctx, world_mm);
        self.dynamic.clear();
        commands
    }

    pub fn on_pointer_secondary_click(
        &mut self,
        ctx: &ToolContext,
        world_mm: DVec2,
    ) -> Vec<AppCommand> {
        let commands = self.tool.on_pointer_secondary_click(ctx, world_mm);
        self.dynamic.clear();
        commands
    }

    pub fn on_escape(&mut self) -> bool {
        if self.dynamic.clear_active_buffer() {
            return true;
        }
        self.tool.on_escape();
        self.dynamic.clear();
        false
    }

    pub fn preview(&self) -> ToolPreview {
        self.dynamic.preview_for(self.tool.as_ref())
    }

    pub fn step_hint(&self) -> String {
        self.tool
            .step_hint()
            .unwrap_or_else(|| self.active_kind.hint().to_string())
    }

    pub fn dynamic_fields(&self) -> &'static [DynamicField] {
        self.tool.dynamic_fields()
    }

    pub fn dynamic_input(&self) -> &DynamicInputState {
        &self.dynamic
    }

    pub fn prepare_dynamic_input(&mut self) -> bool {
        let field_count = self.dynamic_fields().len();
        if field_count == 0 {
            self.dynamic.clear();
            return false;
        }
        self.dynamic.sync(field_count);
        true
    }

    pub fn append_dynamic_chars<I>(&mut self, chars: I)
    where
        I: IntoIterator<Item = char>,
    {
        self.dynamic.append_typed_chars(chars);
    }

    pub fn backspace_dynamic_input(&mut self) -> bool {
        self.dynamic.backspace_active()
    }

    pub fn cycle_dynamic_input(&mut self) {
        self.dynamic.cycle();
    }

    pub fn cycle_dynamic_input_back(&mut self) {
        self.dynamic.cycle_back();
    }

    pub fn commit_dynamic(&mut self, ctx: &ToolContext, world_mm: DVec2) -> Vec<AppCommand> {
        self.dynamic
            .commit_if_valid(self.tool.as_mut(), ctx, world_mm)
    }

    pub fn dynamic_views(&self) -> Vec<DynamicFieldView> {
        self.dynamic.views_for(self.tool.as_ref())
    }
}

fn make_tool(kind: ActiveToolKind) -> Box<dyn Tool> {
    match kind {
        ActiveToolKind::Select => Box::new(SelectTool::default()),
        ActiveToolKind::Pan => Box::new(PassiveTool(kind)),
        ActiveToolKind::Line => Box::new(LineTool::default()),
        ActiveToolKind::Rectangle => Box::new(RectangleTool::default()),
        ActiveToolKind::Circle => Box::new(CircleTool::default()),
        ActiveToolKind::Arc => Box::new(ArcTool::default()),
        ActiveToolKind::Fillet => Box::new(FilletTool::default()),
        ActiveToolKind::Dimension => Box::new(DimensionTool::default()),
        ActiveToolKind::Extrude | ActiveToolKind::Revolve => Box::new(PassiveTool(kind)),
    }
}

struct PassiveTool(ActiveToolKind);

impl Tool for PassiveTool {
    fn kind(&self) -> ActiveToolKind {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::ToolManager;
    use crate::tool::{ActiveToolKind, ToolContext, ToolPreview};

    #[test]
    fn pointer_click_commits_typed_dynamic_values() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut manager = ToolManager::new();
        manager.set_active(ActiveToolKind::Line);

        assert!(manager.on_pointer_click(&ctx, dvec2(0.0, 0.0)).is_empty());
        manager.on_pointer_move(&ctx, dvec2(3.0, 0.0));
        assert!(manager.prepare_dynamic_input());
        manager.append_dynamic_chars("5".chars());

        let commands = manager.on_pointer_click(&ctx, dvec2(3.0, 0.0));

        assert_eq!(
            commands,
            vec![AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 0.0),
            }]
        );
        assert!(manager.dynamic_input().is_empty());
    }

    #[test]
    fn escape_clears_active_dynamic_buffer_before_canceling_tool() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut manager = ToolManager::new();
        manager.set_active(ActiveToolKind::Line);

        assert!(manager.on_pointer_click(&ctx, dvec2(1.0, 1.0)).is_empty());
        manager.on_pointer_move(&ctx, dvec2(4.0, 1.0));
        assert!(manager.prepare_dynamic_input());
        manager.append_dynamic_chars("12".chars());

        assert!(manager.on_escape());
        assert_eq!(manager.dynamic_input().buffer_text(0), Some(""));
        assert!(matches!(manager.preview(), ToolPreview::Line { .. }));

        assert!(!manager.on_escape());
        assert!(matches!(manager.preview(), ToolPreview::None));
    }
}
