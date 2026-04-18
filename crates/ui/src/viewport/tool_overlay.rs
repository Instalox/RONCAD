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
            paint_point_marker(painter, sa, stroke);
            paint_point_marker(painter, sb, stroke);
            let delta = end - start;
            let label = format!(
                "L {} mm\ndX {}   dY {}",
                format_length_mm(start.distance(end)),
                format_length_mm(delta.x.abs()),
                format_length_mm(delta.y.abs())
            );
            paint_label(
                painter,
                Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5),
                Align2::CENTER_BOTTOM,
                &label,
                preview_color,
            );
        }
        ToolPreview::Rectangle { corner_a, corner_b } => {
            paint_rect(painter, camera, center, corner_a, corner_b, stroke);
            let min = corner_a.min(corner_b);
            let max = corner_a.max(corner_b);
            let top_mid = to_pos(camera.world_to_screen(
                DVec2::new((min.x + max.x) * 0.5, max.y),
                center,
            ));
            let right_mid = to_pos(camera.world_to_screen(
                DVec2::new(max.x, (min.y + max.y) * 0.5),
                center,
            ));
            paint_label(
                painter,
                top_mid,
                Align2::CENTER_BOTTOM,
                &format!("W {} mm", format_length_mm((max.x - min.x).abs())),
                preview_color,
            );
            paint_label(
                painter,
                right_mid,
                Align2::LEFT_CENTER,
                &format!("H {} mm", format_length_mm((max.y - min.y).abs())),
                preview_color,
            );
        }
        ToolPreview::Circle { center: c, radius } => {
            let sc = to_pos(camera.world_to_screen(c, center));
            let r_px = (radius * camera.pixels_per_mm) as f32;
            painter.circle_stroke(sc, r_px, stroke);
            painter.line_segment([sc, Pos2::new(sc.x + r_px, sc.y)], stroke);
            paint_point_marker(painter, sc, stroke);
            paint_label(
                painter,
                Pos2::new(sc.x + r_px, sc.y),
                Align2::LEFT_CENTER,
                &format!(
                    "R {} mm\nD {} mm",
                    format_length_mm(radius),
                    format_length_mm(radius * 2.0)
                ),
                preview_color,
            );
        }
        ToolPreview::Measurement { start, end } => {
            let sa = to_pos(camera.world_to_screen(start, center));
            let sb = to_pos(camera.world_to_screen(end, center));
            painter.line_segment([sa, sb], stroke);
            paint_point_marker(painter, sa, stroke);
            paint_point_marker(painter, sb, stroke);
            paint_label(
                painter,
                Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5),
                Align2::CENTER_BOTTOM,
                &format!("{} mm", format_length_mm(start.distance(end))),
                preview_color,
            );
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

fn paint_point_marker(painter: &egui::Painter, point: Pos2, stroke: Stroke) {
    painter.circle_stroke(point, 3.0, stroke);
    painter.circle_filled(point, 1.5, stroke.color);
}

fn paint_label(
    painter: &egui::Painter,
    anchor: Pos2,
    align: Align2,
    text: &str,
    color: egui::Color32,
) {
    let shadow = anchor + egui::vec2(1.0, 1.0);
    let font = FontId::monospace(11.0);
    painter.text(shadow, align, text, font.clone(), egui::Color32::BLACK);
    painter.text(anchor, align, text, font, color);
}
