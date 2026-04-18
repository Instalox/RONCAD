//! Authoritative domain model: sketches, constraints, bodies, features.
//! No UI, no rendering, no file I/O dependencies.

pub mod pick;
pub mod profile;
pub mod project;
pub mod sketch;
pub mod sketch_dimension;
pub mod sketch_entity;
pub mod workplane;

pub use pick::{distance_to_entity, pick_entity};
pub use profile::{SketchProfile, closed_profiles, pick_closed_profile};
pub use project::Project;
pub use sketch::Sketch;
pub use sketch_dimension::SketchDimension;
pub use sketch_entity::SketchEntity;
pub use workplane::Workplane;
