use egui::{Align2, FontId, Stroke};
use roncad_geometry::{arc_sample_points, SketchProfile, Workplane};
use roncad_rendering::Camera2d;

use super::{project_workplane_point, screen_center};
use crate::theme::ThemeColors;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera2d,
    workplane: Option<&Workplane>,
    hovered_profile: Option<&SketchProfile>,
    active_profile: Option<&SketchProfile>,
) {
    let Some(workplane) = workplane else {
        return;
    };
    let center = screen_center(rect);
    if let Some(profile) = active_profile {
        paint_outline(
            painter,
            camera,
            center,
            workplane,
            profile,
            Stroke::new(2.6, ThemeColors::ACCENT.gamma_multiply(0.72)),
        );
    }

    if let Some(profile) = hovered_profile {
        let color = ThemeColors::ACCENT.gamma_multiply(0.92);
        paint_outline(
            painter,
            camera,
            center,
            workplane,
            profile,
            Stroke::new(2.0, color),
        );
        paint_area_label(painter, camera, center, workplane, profile, color);
    } else if let Some(profile) = active_profile {
        paint_area_label(
            painter,
            camera,
            center,
            workplane,
            profile,
            ThemeColors::ACCENT.gamma_multiply(0.78),
        );
    }
}

fn paint_outline(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    workplane: &Workplane,
    profile: &SketchProfile,
    stroke: Stroke,
) {
    match profile {
        SketchProfile::Polygon { points } => {
            let screen_points: Vec<_> = points
                .iter()
                .filter_map(|point| project_workplane_point(camera, center, workplane, *point))
                .collect();
            painter.add(egui::Shape::closed_line(screen_points, stroke));
        }
        SketchProfile::Circle { center: c, radius } => {
            let screen_points: Vec<_> = arc_sample_points(
                *c,
                *radius,
                0.0,
                std::f64::consts::TAU,
                std::f64::consts::PI / 32.0,
            )
            .into_iter()
            .filter_map(|point| project_workplane_point(camera, center, workplane, point))
            .collect();
            painter.add(egui::Shape::closed_line(screen_points, stroke));
        }
    }
}

fn paint_area_label(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    workplane: &Workplane,
    profile: &SketchProfile,
    color: egui::Color32,
) {
    let Some(anchor) = project_workplane_point(camera, center, workplane, profile.centroid())
    else {
        return;
    };
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
