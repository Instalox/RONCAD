//! Selection abstraction: the set of currently selected domain entities.
//! Kept in core so both UI and tools can reason about it without cycles.

use std::collections::HashSet;

use crate::ids::{BodyId, SketchEntityId, SketchId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectionItem {
    Sketch(SketchId),
    SketchEntity {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    Body(BodyId),
}

#[derive(Debug, Default, Clone)]
pub struct Selection {
    items: HashSet<SelectionItem>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn insert(&mut self, item: SelectionItem) -> bool {
        self.items.insert(item)
    }

    pub fn remove(&mut self, item: &SelectionItem) -> bool {
        self.items.remove(item)
    }

    pub fn contains(&self, item: &SelectionItem) -> bool {
        self.items.contains(item)
    }

    pub fn iter(&self) -> impl Iterator<Item = &SelectionItem> {
        self.items.iter()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&SelectionItem) -> bool) {
        self.items.retain(|item| keep(item));
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
