use egui::{Color32, Pos2, Rect, Stroke};
use glam::DVec2;
use roncad_rendering::Camera2d;

use super::{pick_step, screen_center};
use crate::theme::ThemeColors;

pub(super) fn paint(painter: &egui::Painter, rect: Rect, camera: &Camera2d) {
    painter.rect_filled(rect, 0.0, ThemeColors::BG_DEEP);

    let center = screen_center(rect);
    let ppm = camera.pixels_per_mm;

    let minor_step_mm = pick_step(ppm, 8.0);
    let major_step_mm = minor_step_mm * 5.0;

    paint_lines(painter, rect, center, camera, minor_step_mm, ThemeColors::GRID_MINOR, 1.0);
    paint_lines(painter, rect, center, camera, major_step_mm, ThemeColors::GRID_MAJOR, 1.0);

    let origin = camera.world_to_screen(DVec2::ZERO, center);
    if (0.0..=rect.width() as f64).contains(&(origin.x - rect.min.x as f64)) {
        painter.line_segment(
            [Pos2::new(origin.x as f32, rect.min.y), Pos2::new(origin.x as f32, rect.max.y)],
            Stroke::new(1.2, ThemeColors::GRID_AXIS_Y),
        );
    }
    if (0.0..=rect.height() as f64).contains(&(origin.y - rect.min.y as f64)) {
        painter.line_segment(
            [Pos2::new(rect.min.x, origin.y as f32), Pos2::new(rect.max.x, origin.y as f32)],
            Stroke::new(1.2, ThemeColors::GRID_AXIS_X),
        );
    }
}

fn paint_lines(
    painter: &egui::Painter,
    rect: Rect,
    center: DVec2,
    camera: &Camera2d,
    step_mm: f64,
    color: Color32,
    width: f32,
) {
    let stroke = Stroke::new(width, color);
    let step_px = step_mm * camera.pixels_per_mm;
    if step_px < 4.0 {
        return;
    }

    let world_min = camera.screen_to_world(
        DVec2::new(rect.min.x as f64, rect.max.y as f64),
        center,
    );
    let world_max = camera.screen_to_world(
        DVec2::new(rect.max.x as f64, rect.min.y as f64),
        center,
    );

    let x_start = (world_min.x / step_mm).floor() * step_mm;
    let x_end = (world_max.x / step_mm).ceil() * step_mm;
    let mut x = x_start;
    while x <= x_end {
        let p = camera.world_to_screen(DVec2::new(x, 0.0), center);
        let sx = p.x as f32;
        painter.line_segment([Pos2::new(sx, rect.min.y), Pos2::new(sx, rect.max.y)], stroke);
        x += step_mm;
    }

    let y_start = (world_min.y / step_mm).floor() * step_mm;
    let y_end = (world_max.y / step_mm).ceil() * step_mm;
    let mut y = y_start;
    while y <= y_end {
        let p = camera.world_to_screen(DVec2::new(0.0, y), center);
        let sy = p.y as f32;
        painter.line_segment([Pos2::new(rect.min.x, sy), Pos2::new(rect.max.x, sy)], stroke);
        y += step_mm;
    }
}
