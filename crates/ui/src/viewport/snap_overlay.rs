//! Paints a lightweight marker at the active snap point for drawing tools.
//! The marker only appears when the snap engine produced a concrete snap hit.

use egui::{Color32, Pos2, Rect, Stroke};
use roncad_rendering::Camera2d;
use roncad_tools::{SnapKind, SnapResult};

use super::{screen_center, to_pos};

const COLOR_GRID: Color32 = Color32::from_rgb(0x7A, 0xB8, 0xFF);
const COLOR_ENDPOINT: Color32 = Color32::from_rgb(0xFF, 0xD1, 0x66);
const COLOR_CENTER: Color32 = Color32::from_rgb(0x78, 0xE0, 0xA1);

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

    match kind {
        SnapKind::Grid => paint_grid_marker(painter, point),
        SnapKind::Endpoint => paint_endpoint_marker(painter, point),
        SnapKind::Center => paint_center_marker(painter, point),
    }
}

fn paint_grid_marker(painter: &egui::Painter, point: Pos2) {
    let stroke = Stroke::new(1.4, COLOR_GRID);
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

fn paint_endpoint_marker(painter: &egui::Painter, point: Pos2) {
    let stroke = Stroke::new(1.6, COLOR_ENDPOINT);
    painter.circle_stroke(point, 5.0, stroke);
    painter.circle_filled(point, 1.8, COLOR_ENDPOINT);
}

fn paint_center_marker(painter: &egui::Painter, point: Pos2) {
    let stroke = Stroke::new(1.6, COLOR_CENTER);
    painter.circle_stroke(point, 6.0, stroke);
    painter.line_segment(
        [point + egui::vec2(-7.0, 0.0), point + egui::vec2(7.0, 0.0)],
        stroke,
    );
    painter.line_segment(
        [point + egui::vec2(0.0, -7.0), point + egui::vec2(0.0, 7.0)],
        stroke,
    );
}
