//! A 2D sketch bound to a workplane. Owns its entities; constraints land later.

use roncad_core::ids::{SketchEntityId, WorkplaneId};
use slotmap::SlotMap;

use crate::sketch_entity::SketchEntity;

#[derive(Debug, Clone)]
pub struct Sketch {
    pub name: String,
    pub workplane: WorkplaneId,
    pub entities: SlotMap<SketchEntityId, SketchEntity>,
}

impl Sketch {
    pub fn new(name: impl Into<String>, workplane: WorkplaneId) -> Self {
        Self {
            name: name.into(),
            workplane,
            entities: SlotMap::with_key(),
        }
    }

    pub fn add(&mut self, entity: SketchEntity) -> SketchEntityId {
        self.entities.insert(entity)
    }

    pub fn remove(&mut self, id: SketchEntityId) -> Option<SketchEntity> {
        self.entities.remove(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (SketchEntityId, &SketchEntity)> {
        self.entities.iter()
    }
}
