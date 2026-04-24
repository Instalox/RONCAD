//! Command types: user intents to mutate document or selection state.
//! Tools and UI produce commands; a central dispatcher applies them.
//! Commands carry plain data so core stays free of geometry/entity types.

use glam::DVec2;
use serde::{Deserialize, Serialize};

use crate::constraint::{Constraint, EntityPoint};
use crate::ids::{BodyId, ConstraintId, SketchEntityId, SketchId, WorkplaneId};
use crate::units::LengthMm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionEditMode {
    Replace,
    Add,
    Remove,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProfileRegion {
    Polygon { points: Vec<DVec2> },
    Circle { center: DVec2, radius: LengthMm },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppCommand {
    CreateSketch {
        name: String,
        plane: WorkplaneId,
    },
    SetActiveSketch(SketchId),
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
    AddArc {
        sketch: SketchId,
        center: DVec2,
        radius: LengthMm,
        start_angle: f64,
        sweep_angle: f64,
    },
    ApplyLineFillet {
        sketch: SketchId,
        line_a: SketchEntityId,
        line_b: SketchEntityId,
        corner: DVec2,
        radius: LengthMm,
    },
    AddDistanceDimension {
        sketch: SketchId,
        start: DVec2,
        end: DVec2,
    },
    AddConstraint {
        sketch: SketchId,
        constraint: Constraint,
    },
    RemoveConstraint {
        sketch: SketchId,
        constraint: ConstraintId,
    },
    SetLineLength {
        sketch: SketchId,
        entity: SketchEntityId,
        length: LengthMm,
    },
    SetRectangleWidth {
        sketch: SketchId,
        entity: SketchEntityId,
        width: LengthMm,
    },
    SetRectangleHeight {
        sketch: SketchId,
        entity: SketchEntityId,
        height: LengthMm,
    },
    SetCircleRadius {
        sketch: SketchId,
        entity: SketchEntityId,
        radius: LengthMm,
    },
    SetArcRadius {
        sketch: SketchId,
        entity: SketchEntityId,
        radius: LengthMm,
    },
    SetArcSweepDegrees {
        sketch: SketchId,
        entity: SketchEntityId,
        sweep_degrees: f64,
    },
    SetPointX {
        sketch: SketchId,
        entity: SketchEntityId,
        x: f64,
    },
    SetPointY {
        sketch: SketchId,
        entity: SketchEntityId,
        y: f64,
    },
    DeleteEntity {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    SelectSingle {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    SelectEntities {
        sketch: SketchId,
        entities: Vec<SketchEntityId>,
        mode: SelectionEditMode,
    },
    SelectVertices {
        sketch: SketchId,
        points: Vec<EntityPoint>,
        mode: SelectionEditMode,
    },
    SelectBody(BodyId),
    SelectBodies {
        bodies: Vec<BodyId>,
        mode: SelectionEditMode,
    },
    ToggleSelection {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    ClearSelection,
    DeleteSelection,
    ExtrudeProfile {
        sketch: SketchId,
        profile: ProfileRegion,
        distance: LengthMm,
    },
    RevolveProfile {
        sketch: SketchId,
        profile: ProfileRegion,
        axis_origin: DVec2,
        axis_dir: DVec2,
        angle_rad: f64,
    },
    NoOp,
}

pub trait CommandSink {
    fn submit(&mut self, command: AppCommand);
}
