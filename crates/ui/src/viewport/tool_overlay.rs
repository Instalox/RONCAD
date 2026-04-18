use egui::{Rect, Stroke};
use glam::DVec2;
use roncad_rendering::Camera2d;
use roncad_tools::{ToolManager, ToolPreview};

use super::{screen_center, to_pos, COLOR_PREVIEW};

pub(super) fn paint_preview(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    manager: &ToolManager,
) {
    let center = screen_center(rect);
    let stroke = Stroke::new(1.4, COLOR_PREVIEW);
    match manager.preview() {
        ToolPreview::None => {}
        ToolPreview::Line { start, end } => {
            let sa = to_pos(camera.world_to_screen(start, center));
            let sb = to_pos(camera.world_to_screen(end, center));
            painter.line_segment([sa, sb], stroke);
            painter.circle_stroke(sa, 3.0, stroke);
        }
        ToolPreview::Rectangle { corner_a, corner_b } => {
            paint_rect(painter, camera, center, corner_a, corner_b, stroke);
        }
        ToolPreview::Circle { center: c, radius } => {
            let sc = to_pos(camera.world_to_screen(c, center));
            let r_px = (radius * camera.pixels_per_mm) as f32;
            painter.circle_stroke(sc, r_px, stroke);
        }
    }
}

pub(super) fn paint_rect(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    a: DVec2,
    b: DVec2,
    stroke: Stroke,
) {
    let corners = [
        DVec2::new(a.x, a.y),
        DVec2::new(b.x, a.y),
        DVec2::new(b.x, b.y),
        DVec2::new(a.x, b.y),
    ];
    for i in 0..4 {
        let p0 = to_pos(camera.world_to_screen(corners[i], center));
        let p1 = to_pos(camera.world_to_screen(corners[(i + 1) % 4], center));
        painter.line_segment([p0, p1], stroke);
    }
}
