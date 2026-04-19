use egui::{Align2, FontId, Stroke};
use roncad_geometry::SketchProfile;
use roncad_rendering::Camera2d;

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera2d,
    profile: Option<&SketchProfile>,
) {
    let Some(profile) = profile else {
        return;
    };

    let center = screen_center(rect);
    let color = ThemeColors::ACCENT.gamma_multiply(0.92);
    let stroke = Stroke::new(2.0, color);

    match profile {
        SketchProfile::Polygon { points } => {
            let screen_points: Vec<_> = points
                .iter()
                .map(|point| to_pos(camera.world_to_screen(*point, center)))
                .collect();
            painter.add(egui::Shape::closed_line(screen_points, stroke));
        }
        SketchProfile::Circle { center: c, radius } => {
            let screen_center = to_pos(camera.world_to_screen(*c, center));
            painter.circle_stroke(
                screen_center,
                (*radius * camera.pixels_per_mm) as f32,
                stroke,
            );
        }
    }

    let anchor = to_pos(camera.world_to_screen(profile.centroid(), center));
    let label = format!("{:.3} mm^2", profile.area());
    let shadow = anchor + egui::vec2(1.0, 1.0);
    let font = FontId::monospace(11.0);
    painter.text(
        shadow,
        Align2::CENTER_BOTTOM,
        &label,
        font.clone(),
        egui::Color32::BLACK,
    );
    painter.text(anchor, Align2::CENTER_BOTTOM, label, font, color);
}
