use egui::{Color32, FontId, Mesh, Shape, Stroke};
use glam::DVec2;
use roncad_geometry::{arc_sample_points, SketchProfile, Workplane};
use roncad_rendering::{triangulate_polygon, Camera2d};

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
        paint_fill(
            painter,
            camera,
            center,
            workplane,
            profile,
            Color32::from_rgba_premultiplied(0x4F, 0xA3, 0xF7, 0x1C),
        );
        paint_outline(
            painter,
            camera,
            center,
            workplane,
            profile,
            Stroke::new(2.4, ThemeColors::ACCENT.gamma_multiply(0.70)),
        );
    }

    if let Some(profile) = hovered_profile {
        let color = ThemeColors::ACCENT.gamma_multiply(0.92);
        paint_fill(
            painter,
            camera,
            center,
            workplane,
            profile,
            Color32::from_rgba_premultiplied(0x4F, 0xA3, 0xF7, 0x34),
        );
        paint_outline(
            painter,
            camera,
            center,
            workplane,
            profile,
            Stroke::new(2.2, color),
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

fn paint_fill(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    workplane: &Workplane,
    profile: &SketchProfile,
    color: Color32,
) {
    let points = profile_outline_points(profile);
    if points.len() < 3 {
        return;
    }
    let screen_points: Vec<_> = points
        .iter()
        .filter_map(|point| project_workplane_point(camera, center, workplane, *point))
        .collect();
    if screen_points.len() != points.len() {
        return;
    }

    let mut mesh = Mesh::default();
    mesh.reserve_vertices(screen_points.len());
    mesh.reserve_triangles(points.len().saturating_sub(2));
    for point in &screen_points {
        mesh.colored_vertex(*point, color);
    }
    for [a, b, c] in triangulate_polygon(&points) {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    painter.add(Shape::mesh(mesh));
}

fn paint_outline(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    workplane: &Workplane,
    profile: &SketchProfile,
    stroke: Stroke,
) {
    let screen_points: Vec<_> = profile_outline_points(profile)
        .into_iter()
        .filter_map(|point| project_workplane_point(camera, center, workplane, point))
        .collect();
    painter.add(egui::Shape::closed_line(
        screen_points.clone(),
        Stroke::new(stroke.width + 3.2, with_alpha(stroke.color, 40)),
    ));
    painter.add(egui::Shape::closed_line(screen_points, stroke));
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
    let font = FontId::monospace(11.0);
    let galley = painter.layout_no_wrap(label.clone(), font.clone(), color);
    let text_pos = anchor + egui::vec2(-galley.size().x * 0.5, -galley.size().y - 2.0);
    let bg_rect = egui::Rect::from_min_size(
        text_pos - egui::vec2(6.0, 4.0),
        galley.size() + egui::vec2(12.0, 8.0),
    );
    painter.rect_filled(
        bg_rect,
        4.0,
        Color32::from_rgba_premultiplied(0x12, 0x16, 0x1C, 0xD6),
    );
    painter.rect_stroke(
        bg_rect,
        4.0,
        Stroke::new(1.0, with_alpha(color, 64)),
        egui::StrokeKind::Outside,
    );
    painter.galley(
        text_pos + egui::vec2(1.0, 1.0),
        galley.clone(),
        Color32::BLACK,
    );
    painter.galley(text_pos, galley, color);
}

fn profile_outline_points(profile: &SketchProfile) -> Vec<DVec2> {
    match profile {
        SketchProfile::Polygon { points } => points.clone(),
        SketchProfile::Circle { center, radius } => arc_sample_points(
            *center,
            *radius,
            0.0,
            std::f64::consts::TAU,
            std::f64::consts::PI / 32.0,
        ),
    }
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_premultiplied(r, g, b, alpha)
}
