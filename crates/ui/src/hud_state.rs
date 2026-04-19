//! Transient state for the viewport mini HUD. Buffers typed text between
//! frames so the press-and-type workflow keeps user input while the entity
//! remains selected.

use roncad_core::selection::SelectionItem;

#[derive(Debug, Default)]
pub struct HudEditState {
    pub tracked: Option<SelectionItem>,
    pub fields: Vec<String>,
    pub focus_index: Option<usize>,
}

impl HudEditState {
    pub fn clear(&mut self) {
        self.tracked = None;
        self.fields.clear();
        self.focus_index = None;
    }
}
