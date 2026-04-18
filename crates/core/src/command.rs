//! Command types: user intents to mutate document or selection state.
//! Tools and UI produce commands; a central dispatcher applies them.
//! Commands carry plain data so core stays free of geometry/entity types.

use glam::DVec2;
use serde::{Deserialize, Serialize};

use crate::ids::{SketchEntityId, SketchId};
use crate::units::LengthMm;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppCommand {
    CreateSketch {
        name: String,
    },
    DeleteSketch(SketchId),
    AddLine {
        sketch: SketchId,
        a: DVec2,
        b: DVec2,
    },
    AddRectangle {
        sketch: SketchId,
        corner_a: DVec2,
        corner_b: DVec2,
    },
    AddCircle {
        sketch: SketchId,
        center: DVec2,
        radius: LengthMm,
    },
    DeleteEntity {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    SelectSingle {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    ToggleSelection {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    ClearSelection,
    DeleteSelection,
    ExtrudeProfile {
        sketch: SketchId,
        distance: LengthMm,
    },
    NoOp,
}

pub trait CommandSink {
    fn submit(&mut self, command: AppCommand);
}
