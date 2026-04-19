//! Paints the active snap marker plus lightweight reference guides so sketch
//! inference reads directly in the viewport.

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke};
use roncad_rendering::Camera2d;
use roncad_tools::{SnapKind, SnapReference, SnapResult};

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    snap_result: Option<&SnapResult>,
) {
    let Some(snap) = snap_result else {
        return;
    };
    let Some(kind) = snap.kind else {
        return;
    };

    let center = screen_center(rect);
    let point = to_pos(camera.world_to_screen(snap.point, center));

    for reference in snap.references.iter().flatten() {
        paint_reference(painter, camera, center, point, *reference);
    }

    match kind {
        SnapKind::Grid => paint_grid_marker(painter, point, snap_color(kind)),
        SnapKind::Endpoint => paint_endpoint_marker(painter, point, snap_color(kind), 5.0, 1.8),
        SnapKind::Midpoint => paint_midpoint_marker(painter, point, snap_color(kind), 6.0),
        SnapKind::Center => paint_center_marker(painter, point, snap_color(kind), 7.0),
        SnapKind::Horizontal | SnapKind::Vertical | SnapKind::Intersection => {
            paint_alignment_marker(painter, point, snap_color(kind), kind)
        }
    }

    paint_snap_label(painter, point, kind);
}

fn paint_snap_label(painter: &egui::Painter, point: Pos2, kind: SnapKind) {
    let label = kind.label();
    let font = FontId::proportional(10.0);
    let anchor = point + egui::vec2(10.0, -12.0);
    painter.text(
        anchor + egui::vec2(1.0, 1.0),
        Align2::LEFT_BOTTOM,
        label,
        font.clone(),
        Color32::BLACK,
    );
    painter.text(
        anchor,
        Align2::LEFT_BOTTOM,
        label,
        font,
        snap_color(kind).gamma_multiply(0.95),
    );
}

fn paint_reference(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    snap_point: Pos2,
    reference: SnapReference,
) {
    let source = to_pos(camera.world_to_screen(reference.point, center));
    let color = snap_color(reference.kind).gamma_multiply(0.72);

    if reference.axis.is_some() {
        painter.line_segment([source, snap_point], Stroke::new(1.1, color));
    }

    match reference.kind {
        SnapKind::Endpoint => paint_endpoint_marker(painter, source, color, 4.0, 1.2),
        SnapKind::Midpoint => paint_midpoint_marker(painter, source, color, 5.0),
        SnapKind::Center => paint_center_marker(painter, source, color, 5.5),
        SnapKind::Grid | SnapKind::Horizontal | SnapKind::Vertical | SnapKind::Intersection => {}
    }
}

fn snap_color(kind: SnapKind) -> Color32 {
    match kind {
        SnapKind::Grid => ThemeColors::ACCENT.gamma_multiply(0.82),
        SnapKind::Endpoint => ThemeColors::ACCENT_AMBER,
        SnapKind::Midpoint => ThemeColors::ACCENT,
        SnapKind::Center => ThemeColors::GRID_AXIS_Y,
        SnapKind::Horizontal | SnapKind::Vertical | SnapKind::Intersection => ThemeColors::ACCENT,
    }
}

fn paint_grid_marker(painter: &egui::Painter, point: Pos2, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let r = 4.0;
    painter.line_segment(
        [point + egui::vec2(-r, 0.0), point + egui::vec2(r, 0.0)],
        stroke,
    );
    painter.line_segment(
        [point + egui::vec2(0.0, -r), point + egui::vec2(0.0, r)],
        stroke,
    );
    painter.rect_stroke(
        Rect::from_center_size(point, egui::vec2(8.0, 8.0)),
        0.0,
        stroke,
        egui::StrokeKind::Outside,
    );
}

fn paint_endpoint_marker(
    painter: &egui::Painter,
    point: Pos2,
    color: Color32,
    radius: f32,
    fill_radius: f32,
) {
    let stroke = Stroke::new(1.6, color);
    painter.circle_stroke(point, radius, stroke);
    painter.circle_filled(point, fill_radius, color);
}

fn paint_midpoint_marker(painter: &egui::Painter, point: Pos2, color: Color32, radius: f32) {
    let stroke = Stroke::new(1.5, color);
    let top = point + egui::vec2(0.0, -radius);
    let right = point + egui::vec2(radius, 0.0);
    let bottom = point + egui::vec2(0.0, radius);
    let left = point + egui::vec2(-radius, 0.0);
    painter.line_segment([top, right], stroke);
    painter.line_segment([right, bottom], stroke);
    painter.line_segment([bottom, left], stroke);
    painter.line_segment([left, top], stroke);
    painter.circle_filled(point, 1.4, color);
}

fn paint_center_marker(painter: &egui::Painter, point: Pos2, color: Color32, radius: f32) {
    let stroke = Stroke::new(1.6, color);
    painter.circle_stroke(point, radius - 1.0, stroke);
    painter.line_segment(
        [
            point + egui::vec2(-radius, 0.0),
            point + egui::vec2(radius, 0.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            point + egui::vec2(0.0, -radius),
            point + egui::vec2(0.0, radius),
        ],
        stroke,
    );
}

fn paint_alignment_marker(painter: &egui::Painter, point: Pos2, color: Color32, kind: SnapKind) {
    let stroke = Stroke::new(1.5, color);
    painter.rect_stroke(
        Rect::from_center_size(point, egui::vec2(8.0, 8.0)),
        0.0,
        stroke,
        egui::StrokeKind::Outside,
    );
    if matches!(kind, SnapKind::Vertical | SnapKind::Intersection) {
        painter.line_segment(
            [point + egui::vec2(0.0, -9.0), point + egui::vec2(0.0, 9.0)],
            stroke,
        );
    }
    if matches!(kind, SnapKind::Horizontal | SnapKind::Intersection) {
        painter.line_segment(
            [point + egui::vec2(-9.0, 0.0), point + egui::vec2(9.0, 0.0)],
            stroke,
        );
    }
}
