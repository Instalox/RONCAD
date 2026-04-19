//! Fusion-style dynamic input state and helpers. This module owns the typed
//! numeric entry buffers plus parsing, validation, preview shaping, and
//! typed-commit behavior for tools that expose dynamic fields.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{DynamicField, Tool, ToolContext, ToolPreview};

/// Text buffers for the dynamic-input HUD. The interaction controller
/// appends keystrokes to the active buffer; Tab cycles; Enter commits.
#[derive(Debug, Default, Clone)]
pub struct DynamicInputState {
    buffers: Vec<String>,
    active: usize,
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

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn buffer_text(&self, index: usize) -> Option<&str> {
        self.buffers.get(index).map(String::as_str)
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

    pub fn append_typed_chars<I>(&mut self, chars: I)
    where
        I: IntoIterator<Item = char>,
    {
        let Some(buffer) = self.buffers.get_mut(self.active) else {
            return;
        };

        for ch in chars {
            append_typed_char(buffer, ch);
        }
    }

    pub fn backspace_active(&mut self) -> bool {
        let Some(buffer) = self.buffers.get_mut(self.active) else {
            return false;
        };
        buffer.pop().is_some()
    }

    pub fn preview_for(&self, tool: &dyn Tool) -> ToolPreview {
        let values = self.parsed_values();
        tool.dynamic_preview(&values)
            .unwrap_or_else(|| tool.preview())
    }

    pub fn commit_if_valid(
        &mut self,
        tool: &mut dyn Tool,
        ctx: &ToolContext<'_>,
        world_mm: DVec2,
    ) -> Vec<AppCommand> {
        if !self.is_valid_for(tool) {
            return Vec::new();
        }

        let values = self.parsed_values();
        let commands = tool.on_dynamic_commit(ctx, world_mm, &values);
        if !commands.is_empty() {
            self.clear();
        }
        commands
    }

    pub fn views_for(&self, tool: &dyn Tool) -> Vec<DynamicFieldView> {
        let fields = tool.dynamic_fields();
        if fields.is_empty() {
            return Vec::new();
        }

        let parsed_values = self.parsed_values();
        let preview_values = tool.dynamic_display_values(&parsed_values);
        fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let buffer_text = self.buffers.get(index).cloned().unwrap_or_default();
                let parse_state = parse_buffer(&buffer_text);
                let active = index == self.active;
                let (text, state) = match parse_state {
                    DynamicParseState::Empty => (
                        preview_values
                            .get(index)
                            .and_then(|value| *value)
                            .map(|value| format_dynamic_value(field, value))
                            .unwrap_or_else(|| "—".to_string()),
                        DynamicFieldVisualState::Preview,
                    ),
                    DynamicParseState::Incomplete => {
                        (buffer_text, DynamicFieldVisualState::Incomplete)
                    }
                    DynamicParseState::InvalidParse => {
                        (buffer_text, DynamicFieldVisualState::InvalidParse)
                    }
                    DynamicParseState::Parsed(value) => {
                        let state = if tool.dynamic_value_is_valid(index, value) {
                            DynamicFieldVisualState::Valid
                        } else {
                            DynamicFieldVisualState::InvalidGeometry
                        };
                        (buffer_text, state)
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

    fn parsed_values(&self) -> Vec<Option<f64>> {
        self.buffers
            .iter()
            .map(|buffer| match parse_buffer(buffer) {
                DynamicParseState::Parsed(value) => Some(value),
                _ => None,
            })
            .collect()
    }

    fn is_valid_for(&self, tool: &dyn Tool) -> bool {
        self.buffers
            .iter()
            .enumerate()
            .all(|(index, buffer)| match parse_buffer(buffer) {
                DynamicParseState::Empty => true,
                DynamicParseState::Parsed(value) => tool.dynamic_value_is_valid(index, value),
                DynamicParseState::Incomplete | DynamicParseState::InvalidParse => false,
            })
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

fn append_typed_char(buffer: &mut String, ch: char) {
    match ch {
        '0'..='9' => buffer.push(ch),
        '.' => {
            if !buffer.contains('.') {
                buffer.push('.');
            }
        }
        '-' => {
            if buffer.is_empty() {
                buffer.push('-');
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::{DynamicFieldVisualState, DynamicInputState};
    use crate::line_tool::LineTool;
    use crate::tool::{Tool, ToolContext};

    #[test]
    fn commit_if_valid_uses_tool_dynamic_commit_and_clears_buffers() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = LineTool::default();
        let mut state = DynamicInputState::default();

        assert!(tool.on_pointer_click(&ctx, dvec2(0.0, 0.0)).is_empty());
        tool.on_pointer_move(&ctx, dvec2(3.0, 0.0));
        state.sync(2);
        state.append_typed_chars("5".chars());

        let commands = state.commit_if_valid(&mut tool, &ctx, dvec2(3.0, 0.0));

        assert_eq!(
            commands,
            vec![AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 0.0),
            }]
        );
        assert!(state.is_empty());
    }

    #[test]
    fn views_show_preview_value_for_empty_field() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = LineTool::default();
        let mut state = DynamicInputState::default();

        assert!(tool.on_pointer_click(&ctx, dvec2(0.0, 0.0)).is_empty());
        tool.on_pointer_move(&ctx, dvec2(3.0, 4.0));
        state.sync(2);

        let views = state.views_for(&tool);

        assert_eq!(views.len(), 2);
        assert_eq!(views[0].text, "5.000");
        assert_eq!(views[1].text, "53.1");
        assert_eq!(views[0].state, DynamicFieldVisualState::Preview);
        assert_eq!(views[1].state, DynamicFieldVisualState::Preview);
    }

    #[test]
    fn cycle_back_wraps_to_last_field() {
        let mut state = DynamicInputState::default();
        state.sync(3);

        state.cycle_back();
        assert_eq!(state.active_index(), 2);
        state.cycle_back();
        assert_eq!(state.active_index(), 1);
    }

    #[test]
    fn append_typed_chars_accepts_single_decimal_and_leading_minus() {
        let mut state = DynamicInputState::default();
        state.sync(1);
        state.append_typed_chars(['-', '1', '2', '.', '5', '.', '-']);

        assert_eq!(state.buffer_text(0), Some("-12.5"));
    }

    #[test]
    fn append_typed_chars_ignores_non_numeric_input() {
        let mut state = DynamicInputState::default();
        state.sync(1);
        state.append_typed_chars(['x', '3', ' ', '+', '4']);

        assert_eq!(state.buffer_text(0), Some("34"));
    }

    #[test]
    fn backspace_active_removes_last_character() {
        let mut state = DynamicInputState::default();
        state.sync(1);
        state.append_typed_chars("12".chars());

        assert!(state.backspace_active());
        assert_eq!(state.buffer_text(0), Some("1"));
    }
}
