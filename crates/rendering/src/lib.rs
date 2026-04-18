//! GPU rendering and viewport subsystem.
//! wgpu integration lands here; for Milestone 1 the viewport is egui-painted.

pub mod camera;

pub use camera::{adaptive_grid_step_mm, Camera2d};
