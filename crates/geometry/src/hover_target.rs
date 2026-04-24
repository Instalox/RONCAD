//! Typed hover and pick targets for viewport interaction.

use roncad_core::constraint::EntityPoint;
use roncad_core::ids::{SketchEntityId, SketchId};

use crate::SketchProfile;

#[derive(Debug, Clone, PartialEq)]
pub enum HoverTarget {
    SketchEntity {
        sketch: SketchId,
        entity: SketchEntityId,
    },
    SketchVertex {
        sketch: SketchId,
        point: EntityPoint,
    },
    Profile {
        sketch: SketchId,
        profile: SketchProfile,
    },
}

impl HoverTarget {
    pub fn sketch_entity(sketch: SketchId, entity: SketchEntityId) -> Self {
        Self::SketchEntity { sketch, entity }
    }

    pub fn sketch_vertex(sketch: SketchId, point: EntityPoint) -> Self {
        Self::SketchVertex { sketch, point }
    }

    pub fn profile(sketch: SketchId, profile: SketchProfile) -> Self {
        Self::Profile { sketch, profile }
    }

    pub fn sketch_id(&self) -> SketchId {
        match self {
            Self::SketchEntity { sketch, .. }
            | Self::SketchVertex { sketch, .. }
            | Self::Profile { sketch, .. } => *sketch,
        }
    }

    pub fn as_sketch_entity(&self) -> Option<(SketchId, SketchEntityId)> {
        match self {
            Self::SketchEntity { sketch, entity } => Some((*sketch, *entity)),
            Self::SketchVertex { .. } => None,
            Self::Profile { .. } => None,
        }
    }

    pub fn as_sketch_vertex(&self) -> Option<(SketchId, EntityPoint)> {
        match self {
            Self::SketchVertex { sketch, point } => Some((*sketch, *point)),
            _ => None,
        }
    }

    pub fn as_profile(&self) -> Option<&SketchProfile> {
        match self {
            Self::Profile { profile, .. } => Some(profile),
            Self::SketchEntity { .. } | Self::SketchVertex { .. } => None,
        }
    }

    pub fn matches_sketch_entity(&self, sketch: SketchId, entity: SketchEntityId) -> bool {
        matches!(
            self,
            Self::SketchEntity {
                sketch: target_sketch,
                entity: target_entity,
            } if *target_sketch == sketch && *target_entity == entity
        )
    }

    pub fn matches_sketch_vertex(&self, sketch: SketchId, point: EntityPoint) -> bool {
        matches!(
            self,
            Self::SketchVertex {
                sketch: target_sketch,
                point: target_point,
            } if *target_sketch == sketch && *target_point == point
        )
    }
}
