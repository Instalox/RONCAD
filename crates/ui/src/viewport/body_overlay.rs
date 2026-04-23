//! Body rendering. Hands the project's geometry to the wgpu callback,
//! which draws shaded faces and depth-tested edges into an offscreen
//! target before compositing into the egui viewport.

use egui::{Rect, Shape};
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;

use super::wgpu_renderer;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
) {
    let pixels_per_point = painter.ctx().pixels_per_point();
    let callback = wgpu_renderer::build_callback(
        project,
        selection,
        camera,
        rect,
        pixels_per_point,
    );
    let paint_callback = egui_wgpu::Callback::new_paint_callback(rect, callback);
    painter.add(Shape::Callback(paint_callback));
}
