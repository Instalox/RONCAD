//! Routes the active ActiveToolKind to a concrete Tool implementation.
//! The shell owns a ToolManager and the viewport drives it with events.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::arc_tool::ArcTool;
use crate::circle_tool::CircleTool;
use crate::dimension_tool::DimensionTool;
use crate::fillet_tool::FilletTool;
use crate::line_tool::LineTool;
use crate::rectangle_tool::RectangleTool;
use crate::select_tool::SelectTool;
use crate::tool::{ActiveToolKind, DynamicField, Tool, ToolContext, ToolPreview};

/// Text buffers for the Fusion-style dynamic input HUD. The viewport
/// appends keystrokes to the active buffer; Tab cycles; Enter commits.
#[derive(Debug, Default, Clone)]
pub struct DynamicInputState {
    pub buffers: Vec<String>,
    pub active: usize,
}

impl DynamicInputState {
    pub fn clear(&mut self) {
        self.buffers.clear();
        self.active = 0;
    }

    pub fn sync(&mut self, field_count: usize) {
        if self.buffers.len() != field_count {
            self.buffers = vec![String::new(); field_count];
            self.active = 0;
        }
    }

    pub fn cycle(&mut self) {
        if !self.buffers.is_empty() {
            self.active = (self.active + 1) % self.buffers.len();
        }
    }

    pub fn cycle_back(&mut self) {
        if !self.buffers.is_empty() {
            self.active = (self.active + self.buffers.len() - 1) % self.buffers.len();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.iter().all(String::is_empty)
    }

    pub fn has_any_text(&self) -> bool {
        self.buffers.iter().any(|buffer| !buffer.trim().is_empty())
    }

    pub fn active_buffer_mut(&mut self) -> Option<&mut String> {
        self.buffers.get_mut(self.active)
    }

    pub fn clear_active_buffer(&mut self) -> bool {
        let Some(buffer) = self.buffers.get_mut(self.active) else {
            return false;
        };
        if buffer.is_empty() {
            return false;
        }
        buffer.clear();
        true
    }

    pub fn parsed(&self) -> Vec<Option<f64>> {
        self.buffers
            .iter()
            .map(|b| match parse_buffer(b) {
                DynamicParseState::Parsed(value) => Some(value),
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicFieldVisualState {
    Preview,
    Valid,
    Incomplete,
    InvalidParse,
    InvalidGeometry,
}

#[derive(Debug, Clone)]
pub struct DynamicFieldView {
    pub label: &'static str,
    pub unit: &'static str,
    pub text: String,
    pub active: bool,
    pub state: DynamicFieldVisualState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DynamicParseState {
    Empty,
    Incomplete,
    InvalidParse,
    Parsed(f64),
}

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
            if !self.dynamic_input_is_valid() {
                return Vec::new();
            }

            let values = self.dynamic.parsed();
            let commands = self.tool.on_dynamic_commit(ctx, world_mm, &values);
            if !commands.is_empty() {
                self.dynamic.clear();
            }
            return commands;
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
        let values = self.dynamic.parsed();
        self.tool
            .dynamic_preview(&values)
            .unwrap_or_else(|| self.tool.preview())
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

    pub fn dynamic_input_mut(&mut self) -> &mut DynamicInputState {
        &mut self.dynamic
    }

    pub fn commit_dynamic(&mut self, ctx: &ToolContext, world_mm: DVec2) -> Vec<AppCommand> {
        if !self.dynamic_input_is_valid() {
            return Vec::new();
        }
        let values = self.dynamic.parsed();
        let commands = self.tool.on_dynamic_commit(ctx, world_mm, &values);
        if !commands.is_empty() {
            self.dynamic.clear();
        }
        commands
    }

    pub fn dynamic_views(&self) -> Vec<DynamicFieldView> {
        let fields = self.dynamic_fields();
        if fields.is_empty() {
            return Vec::new();
        }

        let parsed_values = self.dynamic.parsed();
        let preview_values = self.tool.dynamic_display_values(&parsed_values);
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let parse_state = self
                    .dynamic
                    .buffers
                    .get(index)
                    .map_or(DynamicParseState::Empty, |buffer| parse_buffer(buffer));
                let active = index == self.dynamic.active;
                let (text, state) = match parse_state {
                    DynamicParseState::Empty => (
                        preview_values
                            .get(index)
                            .and_then(|value| *value)
                            .map(|value| format_dynamic_value(field, value))
                            .unwrap_or_else(|| "—".to_string()),
                        DynamicFieldVisualState::Preview,
                    ),
                    DynamicParseState::Incomplete => (
                        self.dynamic.buffers[index].clone(),
                        DynamicFieldVisualState::Incomplete,
                    ),
                    DynamicParseState::InvalidParse => (
                        self.dynamic.buffers[index].clone(),
                        DynamicFieldVisualState::InvalidParse,
                    ),
                    DynamicParseState::Parsed(value) => {
                        let state = if self.tool.dynamic_value_is_valid(index, value) {
                            DynamicFieldVisualState::Valid
                        } else {
                            DynamicFieldVisualState::InvalidGeometry
                        };
                        (self.dynamic.buffers[index].clone(), state)
                    }
                };

                DynamicFieldView {
                    label: field.label,
                    unit: field.unit,
                    text,
                    active,
                    state,
                }
            })
            .collect()
    }

    fn dynamic_input_is_valid(&self) -> bool {
        self.dynamic
            .buffers
            .iter()
            .enumerate()
            .all(|(index, buffer)| match parse_buffer(buffer) {
                DynamicParseState::Empty => true,
                DynamicParseState::Parsed(value) => self.tool.dynamic_value_is_valid(index, value),
                DynamicParseState::Incomplete | DynamicParseState::InvalidParse => false,
            })
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
        ActiveToolKind::Extrude => Box::new(PassiveTool(kind)),
    }
}

struct PassiveTool(ActiveToolKind);

impl Tool for PassiveTool {
    fn kind(&self) -> ActiveToolKind {
        self.0
    }
}

fn parse_buffer(buffer: &str) -> DynamicParseState {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return DynamicParseState::Empty;
    }
    if matches!(trimmed, "-" | "." | "-.") {
        return DynamicParseState::Incomplete;
    }
    match trimmed.parse::<f64>() {
        Ok(value) if value.is_finite() => DynamicParseState::Parsed(value),
        _ => DynamicParseState::InvalidParse,
    }
}

fn format_dynamic_value(field: &DynamicField, value: f64) -> String {
    match field.unit {
        "deg" => format!("{value:.1}"),
        _ => format!("{value:.3}"),
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::{DynamicInputState, ToolManager};
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
        manager.dynamic_input_mut().sync(2);
        manager.dynamic_input_mut().buffers[0] = "5".to_string();

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
        manager.dynamic_input_mut().sync(2);
        manager.dynamic_input_mut().buffers[0] = "12".to_string();

        assert!(manager.on_escape());
        assert_eq!(manager.dynamic_input().buffers[0], "");
        assert!(matches!(manager.preview(), ToolPreview::Line { .. }));

        assert!(!manager.on_escape());
        assert!(matches!(manager.preview(), ToolPreview::None));
    }

    #[test]
    fn dynamic_input_state_cycles_backwards() {
        let mut state = DynamicInputState {
            buffers: vec![String::new(), String::new(), String::new()],
            active: 0,
        };

        state.cycle_back();
        assert_eq!(state.active, 2);
        state.cycle_back();
        assert_eq!(state.active, 1);
    }
}
