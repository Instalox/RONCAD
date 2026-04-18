use egui::{Align2, FontId, Pos2, Rect, Stroke};
use glam::DVec2;
use roncad_rendering::Camera2d;
use roncad_tools::{ToolManager, ToolPreview};

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

pub(super) fn paint_preview(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    manager: &ToolManager,
) {
    let center = screen_center(rect);
    let preview_color = ThemeColors::tool_accent(manager.active_kind());
    let stroke = Stroke::new(1.4, preview_color);
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
        ToolPreview::Measurement { start, end } => {
            let sa = to_pos(camera.world_to_screen(start, center));
            let sb = to_pos(camera.world_to_screen(end, center));
            painter.line_segment([sa, sb], stroke);
            painter.circle_stroke(sa, 3.0, stroke);
            painter.circle_stroke(sb, 3.0, stroke);

            let label = format!("{} mm", format_length_mm(start.distance(end)));
            let midpoint = Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5);
            let shadow = midpoint + egui::vec2(1.0, 1.0);
            let font = FontId::monospace(11.0);
            painter.text(shadow, Align2::CENTER_BOTTOM, &label, font.clone(), egui::Color32::BLACK);
            painter.text(midpoint, Align2::CENTER_BOTTOM, label, font, preview_color);
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

fn format_length_mm(length_mm: f64) -> String {
    format!("{length_mm:.3}")
}
