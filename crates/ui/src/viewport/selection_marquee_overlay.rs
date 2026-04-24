//! Direction-aware Select marquee overlay.

use egui::{Color32, Pos2, Rect, Stroke};
use roncad_rendering::Camera2d;
use roncad_tools::SelectionMarquee;

use crate::theme::ThemeColors;

use super::{project_workplane_point, screen_center};

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &roncad_geometry::Project,
    marquee: Option<SelectionMarquee>,
) {
    let Some(marquee) = marquee else {
        return;
    };
    let Some(workplane) = project.active_workplane() else {
        return;
    };
    let center = screen_center(rect);
    let Some(start) = project_workplane_point(camera, center, workplane, marquee.start) else {
        return;
    };
    let Some(current) = project_workplane_point(camera, center, workplane, marquee.current) else {
        return;
    };

    let marquee_rect = Rect::from_two_pos(start, current);
    if marquee_rect.width() < 2.0 || marquee_rect.height() < 2.0 {
        return;
    }

    let crossing = marquee.crossing();
    let color = if crossing {
        ThemeColors::ACCENT_GREEN
    } else {
        ThemeColors::ACCENT
    };
    let fill = with_alpha(color, if crossing { 26 } else { 20 });
    painter.rect_filled(marquee_rect, 2.0, fill);

    if crossing {
        paint_dashed_rect(painter, marquee_rect, Stroke::new(1.5, color));
    } else {
        painter.rect_stroke(
            marquee_rect,
            2.0,
            Stroke::new(1.5, color),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn paint_lasso(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &roncad_geometry::Project,
    points: Option<&[glam::DVec2]>,
) {
    let Some(points) = points else {
        return;
    };
    if points.len() < 2 {
        return;
    }
    let Some(workplane) = project.active_workplane() else {
        return;
    };
    let center = screen_center(rect);
    let screen_points: Vec<_> = points
        .iter()
        .filter_map(|point| project_workplane_point(camera, center, workplane, *point))
        .collect();
    if screen_points.len() < 2 {
        return;
    }

    let color = ThemeColors::ACCENT_AMBER;
    for pair in screen_points.windows(2) {
        paint_dashed_segment(painter, pair[0], pair[1], Stroke::new(1.5, color));
    }
    if let (Some(first), Some(last)) = (screen_points.first(), screen_points.last()) {
        paint_dashed_segment(painter, *last, *first, Stroke::new(1.5, color));
    }
}

fn paint_dashed_rect(painter: &egui::Painter, rect: Rect, stroke: Stroke) {
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    for i in 0..4 {
        paint_dashed_segment(painter, corners[i], corners[(i + 1) % 4], stroke);
    }
}

fn paint_dashed_segment(painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    let delta = b - a;
    let length = delta.length();
    if length <= f32::EPSILON {
        return;
    }
    let dir = delta / length;
    let dash = 8.0;
    let gap = 5.0;
    let mut t = 0.0;
    while t < length {
        let end = (t + dash).min(length);
        painter.line_segment([a + dir * t, a + dir * end], stroke);
        t += dash + gap;
    }
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_premultiplied(r, g, b, alpha)
}
