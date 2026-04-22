//! GPU rendering and viewport subsystem.
//! wgpu integration lands here; for Milestone 1 the viewport is egui-painted.

pub mod body_mesh;
pub mod camera;

pub use body_mesh::{extrude_mesh, EdgeKind, ExtrudeMesh3d, MeshEdge3d, MeshTriangle3d, MeshVertex3d};
pub use camera::{adaptive_grid_step_mm, Camera2d, Projection};
