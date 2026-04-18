//! Event types: derived notifications emitted after state transitions.
//! Observers (UI, render cache) react to events; they never mutate state directly.

use crate::ids::{BodyId, SketchId};

#[derive(Debug, Clone)]
pub enum AppEvent {
    SelectionChanged,
    DocumentDirtyChanged(bool),
    FeatureRebuilt(SketchId),
    MeshInvalidated(BodyId),
}
