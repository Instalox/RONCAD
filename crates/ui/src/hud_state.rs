//! Transient state for the viewport mini HUD. Buffers typed text between
//! frames so the press-and-type workflow keeps user input while the entity
//! remains selected.

use roncad_core::ids::{SketchEntityId, SketchId};

#[derive(Debug, Default)]
pub struct HudEditState {
    pub tracked: Option<(SketchId, SketchEntityId)>,
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
