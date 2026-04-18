//! The authoritative Project model: workplanes and sketches today; bodies and
//! features arrive in later milestones.

use roncad_core::ids::{SketchId, WorkplaneId};
use slotmap::SlotMap;

use crate::sketch::Sketch;
use crate::workplane::Workplane;

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub workplanes: SlotMap<WorkplaneId, Workplane>,
    pub sketches: SlotMap<SketchId, Sketch>,
    pub active_sketch: Option<SketchId>,
}

impl Default for Project {
    fn default() -> Self {
        Self::new_untitled()
    }
}

impl Project {
    pub fn new_untitled() -> Self {
        let mut workplanes = SlotMap::with_key();
        let xy = workplanes.insert(Workplane::xy());

        let mut sketches = SlotMap::with_key();
        let first = sketches.insert(Sketch::new("Sketch 1", xy));

        Self {
            name: "Untitled".to_string(),
            workplanes,
            sketches,
            active_sketch: Some(first),
        }
    }

    pub fn active_sketch(&self) -> Option<&Sketch> {
        self.active_sketch.and_then(|id| self.sketches.get(id))
    }

    pub fn active_sketch_mut(&mut self) -> Option<&mut Sketch> {
        match self.active_sketch {
            Some(id) => self.sketches.get_mut(id),
            None => None,
        }
    }
}
