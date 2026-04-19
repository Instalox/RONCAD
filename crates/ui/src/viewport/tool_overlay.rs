use std::f64::consts::PI;

use egui::{Align2, FontId, Pos2, Rect, Stroke};
use glam::DVec2;
use roncad_geometry::{arc_mid_point, arc_sample_points};
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
            let (anchor, align) = line_label_placement(sa, sb);
            paint_label(
                painter,
                anchor,
                align,
                &format!("L {} mm", format_length_mm(start.distance(end))),
                preview_color,
            );
        }
        ToolPreview::Rectangle { corner_a, corner_b } => {
            paint_rect(painter, camera, center, corner_a, corner_b, stroke);
            let min = corner_a.min(corner_b);
            let max = corner_a.max(corner_b);
            let top_mid =
                to_pos(camera.world_to_screen(DVec2::new((min.x + max.x) * 0.5, max.y), center));
            let right_mid =
                to_pos(camera.world_to_screen(DVec2::new(max.x, (min.y + max.y) * 0.5), center));
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
        ToolPreview::FilletHover {
            corner,
            trim_a,
            trim_b,
            center: arc_center,
            radius,
            start_angle,
            sweep_angle,
            max_radius,
        } => {
            let hover_color = preview_color.gamma_multiply(0.82);
            let hover_stroke = Stroke::new(1.6, hover_color);
            paint_fillet_geometry(
                painter,
                camera,
                center,
                trim_a,
                trim_b,
                arc_center,
                radius,
                start_angle,
                sweep_angle,
                hover_stroke,
            );

            let corner_pos = to_pos(camera.world_to_screen(corner, center));
            painter.circle_stroke(corner_pos, 7.0, Stroke::new(1.3, hover_color));
            painter.circle_filled(corner_pos, 2.0, hover_color);
            paint_label(
                painter,
                corner_pos + egui::vec2(12.0, -10.0),
                Align2::LEFT_BOTTOM,
                &format!("Fillet\nR<= {} mm", format_length_mm(max_radius)),
                hover_color,
            );
        }
        ToolPreview::Fillet {
            trim_a,
            trim_b,
            center: arc_center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            paint_fillet_geometry(
                painter,
                camera,
                center,
                trim_a,
                trim_b,
                arc_center,
                radius,
                start_angle,
                sweep_angle,
                stroke,
            );

            let center_pos = to_pos(camera.world_to_screen(arc_center, center));
            let mid_world = arc_mid_point(arc_center, radius, start_angle, sweep_angle);
            let mid_pos = to_pos(camera.world_to_screen(mid_world, center));
            let (anchor, align) = line_label_placement(center_pos, mid_pos);
            paint_label(
                painter,
                anchor,
                align,
                &format!("R {} mm", format_length_mm(radius)),
                preview_color,
            );
        }
        ToolPreview::Measurement { start, end } => {
            let sa = to_pos(camera.world_to_screen(start, center));
            let sb = to_pos(camera.world_to_screen(end, center));
            painter.line_segment([sa, sb], stroke);
            paint_point_marker(painter, sa, stroke);
            paint_point_marker(painter, sb, stroke);
            let (anchor, align) = line_label_placement(sa, sb);
            paint_label(
                painter,
                anchor,
                align,
                &format!("{} mm", format_length_mm(start.distance(end))),
                preview_color,
            );
        }
    }
}

fn paint_fillet_geometry(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    trim_a: (DVec2, DVec2),
    trim_b: (DVec2, DVec2),
    arc_center: DVec2,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    stroke: Stroke,
) {
    let trim_a_start = to_pos(camera.world_to_screen(trim_a.0, center));
    let trim_a_end = to_pos(camera.world_to_screen(trim_a.1, center));
    let trim_b_start = to_pos(camera.world_to_screen(trim_b.0, center));
    let trim_b_end = to_pos(camera.world_to_screen(trim_b.1, center));
    painter.line_segment([trim_a_start, trim_a_end], stroke);
    painter.line_segment([trim_b_start, trim_b_end], stroke);

    let arc_points: Vec<_> =
        arc_sample_points(arc_center, radius, start_angle, sweep_angle, PI / 48.0)
            .into_iter()
            .map(|point| to_pos(camera.world_to_screen(point, center)))
            .collect();
    paint_polyline(painter, &arc_points, stroke);

    let center_pos = to_pos(camera.world_to_screen(arc_center, center));
    let mid_world = arc_mid_point(arc_center, radius, start_angle, sweep_angle);
    let mid_pos = to_pos(camera.world_to_screen(mid_world, center));
    painter.line_segment([center_pos, mid_pos], stroke);
    paint_point_marker(painter, trim_a_end, stroke);
    paint_point_marker(painter, trim_b_end, stroke);
    paint_point_marker(painter, center_pos, stroke);
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

fn paint_polyline(painter: &egui::Painter, points: &[Pos2], stroke: Stroke) {
    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], stroke);
    }
}

fn format_length_mm(length_mm: f64) -> String {
    format!("{length_mm:.3}")
}

fn line_label_placement(start: Pos2, end: Pos2) -> (Pos2, Align2) {
    let delta = end - start;
    let midpoint = start + delta * 0.5;
    if delta.length_sq() <= f32::EPSILON {
        return (midpoint + egui::vec2(0.0, -18.0), Align2::CENTER_BOTTOM);
    }

    let len = delta.length();
    let mut normal = egui::vec2(-delta.y / len, delta.x / len);
    if normal.y > 0.0 || (normal.y.abs() < 0.15 && normal.x > 0.0) {
        normal = -normal;
    }

    let anchor = midpoint + normal * 18.0;
    let align = if normal.y <= -0.4 {
        Align2::CENTER_BOTTOM
    } else if normal.x < 0.0 {
        Align2::RIGHT_CENTER
    } else {
        Align2::LEFT_CENTER
    };
    (anchor, align)
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

#[cfg(test)]
mod tests {
    use egui::{Align2, Pos2};

    use super::line_label_placement;

    #[test]
    fn shallow_line_label_sits_above_segment() {
        let start = Pos2::new(20.0, 40.0);
        let end = Pos2::new(80.0, 55.0);
        let midpoint = Pos2::new(50.0, 47.5);

        let (anchor, align) = line_label_placement(start, end);

        assert!(anchor.y < midpoint.y);
        assert_eq!(align, Align2::CENTER_BOTTOM);
    }

    #[test]
    fn vertical_line_label_sits_left_of_segment() {
        let start = Pos2::new(64.0, 20.0);
        let end = Pos2::new(64.0, 80.0);
        let midpoint = Pos2::new(64.0, 50.0);

        let (anchor, align) = line_label_placement(start, end);

        assert!(anchor.x < midpoint.x);
        assert_eq!(align, Align2::RIGHT_CENTER);
    }
}
