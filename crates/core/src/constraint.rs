//! Constraint data types.
//!
//! Relationships between sketch entities. Pure ID references — no geometry
//! math here (that lives in the geometry crate alongside the solver). Kept
//! in core so AppCommand can carry constraint values without pulling in a
//! geometry dependency.

use glam::DVec2;
use serde::{Deserialize, Serialize};

use crate::ids::SketchEntityId;

/// Names a well-defined point on a sketch entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityPoint {
    /// Position of a standalone point entity.
    Point(SketchEntityId),
    /// Start of a line (point `a`) or start of an arc's sweep.
    Start(SketchEntityId),
    /// End of a line (point `b`) or end of an arc's sweep.
    End(SketchEntityId),
    /// Center of a circle or arc.
    Center(SketchEntityId),
    /// Rectangle corner_a.
    CornerA(SketchEntityId),
    /// Rectangle corner at (corner_b.x, corner_a.y).
    CornerB(SketchEntityId),
    /// Rectangle corner_b.
    CornerC(SketchEntityId),
    /// Rectangle corner at (corner_a.x, corner_b.y).
    CornerD(SketchEntityId),
}

impl EntityPoint {
    pub fn entity(self) -> SketchEntityId {
        match self {
            Self::Point(id)
            | Self::Start(id)
            | Self::End(id)
            | Self::Center(id)
            | Self::CornerA(id)
            | Self::CornerB(id)
            | Self::CornerC(id)
            | Self::CornerD(id) => id,
        }
    }
}

/// A geometric relationship between sketch entities. Recorded at insert
/// time (via inference) or by explicit user action. The solver reads these
/// and drives geometry to satisfy them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Constraint {
    /// Two named entity points share a location.
    Coincident { a: EntityPoint, b: EntityPoint },
    /// A named entity point stays at a fixed location.
    FixPoint { point: EntityPoint, target: DVec2 },
    /// A named entity point lies on the body of another entity.
    PointOnEntity {
        point: EntityPoint,
        entity: SketchEntityId,
    },
    /// A line is horizontal (its two endpoints share y).
    Horizontal { entity: SketchEntityId },
    /// A line is vertical (its two endpoints share x).
    Vertical { entity: SketchEntityId },
    /// Two lines share direction (cross product of direction vectors == 0).
    Parallel {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    /// Two lines meet at a right angle (dot product of direction vectors == 0).
    Perpendicular {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    /// A line is tangent to a circle or arc.
    Tangent {
        line: SketchEntityId,
        curve: SketchEntityId,
    },
    /// Two lines have the same length.
    EqualLength {
        a: SketchEntityId,
        b: SketchEntityId,
    },
    /// Two circles or arcs have the same radius.
    EqualRadius {
        a: SketchEntityId,
        b: SketchEntityId,
    },
}

impl Constraint {
    /// Every entity this constraint references. Useful for cascade-delete
    /// when an entity is removed from the sketch.
    pub fn referenced_entities(&self) -> Vec<SketchEntityId> {
        match self {
            Self::Coincident { a, b } => vec![a.entity(), b.entity()],
            Self::FixPoint { point, .. } => vec![point.entity()],
            Self::PointOnEntity { point, entity } => vec![point.entity(), *entity],
            Self::Horizontal { entity } | Self::Vertical { entity } => vec![*entity],
            Self::Parallel { a, b }
            | Self::Perpendicular { a, b }
            | Self::EqualLength { a, b }
            | Self::EqualRadius { a, b } => vec![*a, *b],
            Self::Tangent { line, curve } => vec![*line, *curve],
        }
    }
}
