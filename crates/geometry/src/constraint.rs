//! Geometry-level helpers for constraint types. The types themselves live
//! in `roncad_core::constraint` so they can flow through commands without
//! dragging geometry along.

use glam::DVec2;

use crate::sketch_entity::SketchEntity;
use crate::{arc_end_point, arc_start_point};

pub use roncad_core::constraint::{Constraint, EntityPoint};

/// Resolve an entity-point handle to its current world-space location.
/// Returns `None` if the handle doesn't name a real point on `entity`
/// (e.g., asking for `Center` of a line).
pub fn resolve_entity_point(handle: EntityPoint, entity: &SketchEntity) -> Option<DVec2> {
    match (handle, entity) {
        (EntityPoint::Point(_), SketchEntity::Point { p }) => Some(*p),
        (EntityPoint::Start(_), SketchEntity::Line { a, .. }) => Some(*a),
        (EntityPoint::End(_), SketchEntity::Line { b, .. }) => Some(*b),
        (
            EntityPoint::Start(_),
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                ..
            },
        ) => Some(arc_start_point(*center, *radius, *start_angle)),
        (
            EntityPoint::End(_),
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            },
        ) => Some(arc_end_point(*center, *radius, *start_angle, *sweep_angle)),
        (EntityPoint::Center(_), SketchEntity::Circle { center, .. }) => Some(*center),
        (EntityPoint::Center(_), SketchEntity::Arc { center, .. }) => Some(*center),
        _ => None,
    }
}
