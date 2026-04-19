use egui::{Color32, Pos2, Rect, Stroke};
use glam::DVec2;
use roncad_rendering::Camera2d;

use super::{pick_step, screen_center};
use crate::theme::ThemeColors;

const DOT_GRID_TARGET_SPACING_PX: f64 = 12.0;
const MINOR_DOT_MIN_SPACING_PX: f64 = 6.0;
const MINOR_DOT_FULL_SPACING_PX: f64 = 18.0;
const MAJOR_DOT_MIN_SPACING_PX: f64 = 10.0;
const MAJOR_DOT_FULL_SPACING_PX: f64 = 26.0;
const MINOR_DOT_RADIUS: f32 = 0.85;
const MAJOR_DOT_RADIUS: f32 = 1.4;

pub(super) fn paint(painter: &egui::Painter, rect: Rect, camera: &Camera2d) {
    painter.rect_filled(rect, 0.0, ThemeColors::BG_DEEP);

    let center = screen_center(rect);
    let ppm = camera.pixels_per_mm;

    let minor_step_mm = pick_step(ppm, DOT_GRID_TARGET_SPACING_PX);
    let major_step_mm = minor_step_mm * 5.0;

    paint_dots(
        painter,
        rect,
        center,
        camera,
        minor_step_mm,
        ThemeColors::GRID_DOT,
        MINOR_DOT_RADIUS,
        MINOR_DOT_MIN_SPACING_PX,
        MINOR_DOT_FULL_SPACING_PX,
    );
    paint_dots(
        painter,
        rect,
        center,
        camera,
        major_step_mm,
        ThemeColors::GRID_MAJOR,
        MAJOR_DOT_RADIUS,
        MAJOR_DOT_MIN_SPACING_PX,
        MAJOR_DOT_FULL_SPACING_PX,
    );

    paint_axes(painter, rect, center, camera);
}

fn paint_dots(
    painter: &egui::Painter,
    rect: Rect,
    center: DVec2,
    camera: &Camera2d,
    step_mm: f64,
    color: Color32,
    base_radius: f32,
    min_spacing_px: f64,
    full_spacing_px: f64,
) {
    let step_px = step_mm * camera.pixels_per_mm;
    let Some(strength) = dot_strength(step_px, min_spacing_px, full_spacing_px) else {
        return;
    };

    let (world_min, world_max) = world_bounds(rect, center, camera);
    let x_start = (world_min.x / step_mm).floor() * step_mm;
    let x_end = (world_max.x / step_mm).ceil() * step_mm;
    let y_start = (world_min.y / step_mm).floor() * step_mm;
    let y_end = (world_max.y / step_mm).ceil() * step_mm;

    let color = color.gamma_multiply(0.4 + 0.6 * strength);
    let radius = base_radius * (0.9 + 0.1 * strength);

    let mut x = x_start;
    while x <= x_end {
        let mut y = y_start;
        while y <= y_end {
            let p = camera.world_to_screen(DVec2::new(x, y), center);
            let pos = Pos2::new(p.x as f32, p.y as f32);
            if rect.contains(pos) {
                painter.circle_filled(pos, radius, color);
            }
            y += step_mm;
        }
        x += step_mm;
    }
}

fn paint_axes(painter: &egui::Painter, rect: Rect, center: DVec2, camera: &Camera2d) {
    let origin = camera.world_to_screen(DVec2::ZERO, center);
    if (rect.min.x as f64..=rect.max.x as f64).contains(&origin.x) {
        painter.line_segment(
            [
                Pos2::new(origin.x as f32, rect.min.y),
                Pos2::new(origin.x as f32, rect.max.y),
            ],
            Stroke::new(1.2, ThemeColors::GRID_AXIS_Y),
        );
    }
    if (rect.min.y as f64..=rect.max.y as f64).contains(&origin.y) {
        painter.line_segment(
            [
                Pos2::new(rect.min.x, origin.y as f32),
                Pos2::new(rect.max.x, origin.y as f32),
            ],
            Stroke::new(1.2, ThemeColors::GRID_AXIS_X),
        );
    }
}

fn world_bounds(rect: Rect, center: DVec2, camera: &Camera2d) -> (DVec2, DVec2) {
    let world_min =
        camera.screen_to_world(DVec2::new(rect.min.x as f64, rect.max.y as f64), center);
    let world_max =
        camera.screen_to_world(DVec2::new(rect.max.x as f64, rect.min.y as f64), center);
    (world_min, world_max)
}

fn dot_strength(step_px: f64, min_spacing_px: f64, full_spacing_px: f64) -> Option<f32> {
    if step_px < min_spacing_px {
        return None;
    }
    if step_px >= full_spacing_px {
        return Some(1.0);
    }
    let strength = (step_px - min_spacing_px) / (full_spacing_px - min_spacing_px);
    Some(strength.clamp(0.0, 1.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::dot_strength;
    use crate::viewport::pick_step;

    #[test]
    fn dot_grid_prefers_sparser_step_at_default_zoom() {
        assert_eq!(pick_step(5.0, 12.0), 5.0);
    }

    #[test]
    fn dot_strength_hides_dense_spacing() {
        assert_eq!(dot_strength(5.9, 6.0, 18.0), None);
    }

    #[test]
    fn dot_strength_reaches_full_opacity_after_target_spacing() {
        assert_eq!(dot_strength(18.0, 6.0, 18.0), Some(1.0));
    }

    #[test]
    fn dot_strength_interpolates_between_thresholds() {
        assert_eq!(dot_strength(12.0, 6.0, 18.0), Some(0.5));
    }
}
