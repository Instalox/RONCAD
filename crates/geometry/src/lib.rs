//! Authoritative domain model: sketches, constraints, bodies, features.
//! No UI, no rendering, no file I/O dependencies.

pub mod arc;
pub mod body;
pub mod constraint;
pub mod constraint_inference;
pub mod feature;
pub mod fillet;
pub mod hover_target;
pub mod pick;
pub mod profile;
pub mod project;
pub mod sketch;
pub mod sketch_dimension;
pub mod sketch_entity;
pub mod solver;
pub mod topology;
pub mod workplane;

pub use arc::{
    arc_contains_angle, arc_end_angle, arc_end_point, arc_mid_point, arc_point, arc_sample_points,
    arc_start_point, distance_to_arc,
};
pub use body::Body;
pub use constraint::{resolve_entity_point, Constraint, EntityPoint};
pub use constraint_inference::{infer_constraints, INFERENCE_EPSILON};
pub use feature::{ExtrudeFeature, Feature};
pub use fillet::{
    apply_line_fillet, fillet_candidate_for_lines, find_line_fillet_candidate,
    LineFilletApplyResult, LineFilletCandidate, LineFilletPreview,
};
pub use hover_target::HoverTarget;
pub use pick::{distance_to_entity, pick_entities_stack, pick_entity};
pub use profile::{closed_profiles, pick_closed_profile, SketchProfile};
pub use project::Project;
pub use sketch::Sketch;
pub use sketch_dimension::SketchDimension;
pub use sketch_entity::SketchEntity;
pub use solver::{
    solve_sketch, solve_sketch_with, ConstraintDiagnostic, ConstraintDiagnosticKind, SolveReport,
    SolveStatus,
};
pub use topology::{ProfileKey, ProfileSpanKey, SketchTopology, TopologyEdge, TopologyProfile};
pub use workplane::Workplane;
