//! Overlap-stack preselection for Select.
//!
//! Tracks the set of entities under the cursor, sorted nearest-first, plus an
//! index the user can advance with Tab to cycle through items at the same
//! cursor location. The viewport's hover preview reflects the current index.

use glam::DVec2;
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_geometry::{pick_entities_stack, HoverTarget, Sketch};

/// Cursor movement beyond this rebuilds the stack. Keeps the user's cycled
/// choice stable against sub-pixel jitter while feeling responsive to real moves.
const REBUILD_THRESHOLD_MM: f64 = 0.5;

#[derive(Default)]
pub struct PreselectionState {
    sketch: Option<SketchId>,
    stack: Vec<SketchEntityId>,
    index: usize,
    anchor: Option<DVec2>,
}

impl PreselectionState {
    pub fn clear(&mut self) {
        self.sketch = None;
        self.stack.clear();
        self.index = 0;
        self.anchor = None;
    }

    /// Rebuild the stack if the cursor has moved meaningfully or the sketch
    /// changed. A no-op if the cursor is close to the last anchor, preserving
    /// whatever index the user may have cycled to.
    pub fn update(
        &mut self,
        sketch_id: Option<SketchId>,
        sketch: Option<&Sketch>,
        world: Option<DVec2>,
        tolerance_mm: f64,
    ) {
        let (Some(sketch_id), Some(sketch), Some(world)) = (sketch_id, sketch, world) else {
            self.clear();
            return;
        };

        let sketch_changed = self.sketch != Some(sketch_id);
        let anchor_moved = self
            .anchor
            .is_none_or(|anchor| anchor.distance(world) > REBUILD_THRESHOLD_MM);

        if sketch_changed || anchor_moved {
            self.stack = pick_entities_stack(sketch, world, tolerance_mm);
            self.index = 0;
            self.anchor = Some(world);
            self.sketch = Some(sketch_id);
        }
    }

    /// Advance to the next entity in the stack. Wraps. No-op if stack < 2.
    pub fn cycle(&mut self) {
        if self.stack.len() < 2 {
            return;
        }
        self.index = (self.index + 1) % self.stack.len();
    }

    pub fn current(&self) -> Option<(SketchId, SketchEntityId)> {
        let sketch = self.sketch?;
        self.stack.get(self.index).map(|id| (sketch, *id))
    }

    pub fn hover_target(&self) -> Option<HoverTarget> {
        self.current()
            .map(|(s, e)| HoverTarget::sketch_entity(s, e))
    }

    pub fn stack_size(&self) -> usize {
        self.stack.len()
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec2;
    use roncad_geometry::{Project, SketchEntity};

    fn setup() -> (Project, SketchId) {
        let project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("default sketch");
        (project, sketch_id)
    }

    #[test]
    fn no_cursor_clears_state() {
        let (project, sketch_id) = setup();
        let mut state = PreselectionState::default();
        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(0.0, 0.0)),
            1.0,
        );
        state.update(None, None, None, 1.0);
        assert_eq!(state.stack_size(), 0);
        assert!(state.current().is_none());
    }

    #[test]
    fn cycle_walks_the_overlap_stack() {
        let (mut project, sketch_id) = setup();
        let sketch = project.active_sketch_mut().expect("active sketch");
        let closer = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        let farther = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.4),
            b: dvec2(10.0, 0.4),
        });

        let mut state = PreselectionState::default();
        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(5.0, 0.0)),
            1.0,
        );

        assert_eq!(state.current(), Some((sketch_id, closer)));
        state.cycle();
        assert_eq!(state.current(), Some((sketch_id, farther)));
        state.cycle();
        assert_eq!(state.current(), Some((sketch_id, closer)));
    }

    #[test]
    fn small_cursor_jitter_preserves_cycle_index() {
        let (mut project, sketch_id) = setup();
        let sketch = project.active_sketch_mut().expect("active sketch");
        let a = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        let b = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.3),
            b: dvec2(10.0, 0.3),
        });

        let mut state = PreselectionState::default();
        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(5.0, 0.1)),
            1.0,
        );
        state.cycle();
        let after_cycle = state.current();
        assert_eq!(after_cycle, Some((sketch_id, b)));

        // Sub-threshold jitter: anchor unchanged, index preserved.
        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(5.2, 0.1)),
            1.0,
        );
        assert_eq!(state.current(), after_cycle);

        // Past threshold: stack rebuilds, index resets.
        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(6.0, 0.1)),
            1.0,
        );
        assert_eq!(state.current(), Some((sketch_id, a)));
    }

    #[test]
    fn cycle_is_no_op_on_empty_or_single_stack() {
        let (mut project, sketch_id) = setup();
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });

        let mut state = PreselectionState::default();
        state.cycle();
        assert!(state.current().is_none());

        state.update(
            Some(sketch_id),
            project.active_sketch(),
            Some(dvec2(5.0, 0.0)),
            1.0,
        );
        let before = state.current();
        state.cycle();
        assert_eq!(state.current(), before);
    }
}
