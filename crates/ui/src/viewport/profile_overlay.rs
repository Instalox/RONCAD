use egui::{Align2, FontId, Stroke};
use roncad_geometry::SketchProfile;
use roncad_rendering::Camera2d;

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera2d,
    hovered_profile: Option<&SketchProfile>,
    active_profile: Option<&SketchProfile>,
) {
    let center = screen_center(rect);
    if let Some(profile) = active_profile {
        paint_outline(
            painter,
            camera,
            center,
            profile,
            Stroke::new(2.6, ThemeColors::ACCENT.gamma_multiply(0.72)),
        );
    }

    if let Some(profile) = hovered_profile {
        let color = ThemeColors::ACCENT.gamma_multiply(0.92);
        paint_outline(painter, camera, center, profile, Stroke::new(2.0, color));
        paint_area_label(painter, camera, center, profile, color);
    } else if let Some(profile) = active_profile {
        paint_area_label(
            painter,
            camera,
            center,
            profile,
            ThemeColors::ACCENT.gamma_multiply(0.78),
        );
    }
}

fn paint_outline(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    profile: &SketchProfile,
    stroke: Stroke,
) {
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
}

fn paint_area_label(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    profile: &SketchProfile,
    color: egui::Color32,
) {
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
