//! 3D Navigation Gizmo overlay (View Cube).
//! Placed in the top-right corner to show camera orientation and allow snapping to standard views.

use egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2};
use glam::DVec3;
use std::f64::consts::{FRAC_PI_2, PI};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const CUBE_RADIUS: f32 = 32.0;
const GIZMO_PADDING: f32 = 24.0;

#[derive(Clone, Copy)]
struct Face {
    label: &'static str,
    normal: DVec3,
    yaw: f64,
    pitch: f64,
    corners: [DVec3; 4],
}

fn create_faces() -> [Face; 6] {
    let top_corners = [
        DVec3::new(-1.0, -1.0, 1.0),
        DVec3::new(1.0, -1.0, 1.0),
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(-1.0, 1.0, 1.0),
    ];
    let bottom_corners = [
        DVec3::new(-1.0, 1.0, -1.0),
        DVec3::new(1.0, 1.0, -1.0),
        DVec3::new(1.0, -1.0, -1.0),
        DVec3::new(-1.0, -1.0, -1.0),
    ];
    let front_corners = [
        DVec3::new(-1.0, -1.0, -1.0),
        DVec3::new(1.0, -1.0, -1.0),
        DVec3::new(1.0, -1.0, 1.0),
        DVec3::new(-1.0, -1.0, 1.0),
    ];
    let back_corners = [
        DVec3::new(1.0, 1.0, -1.0),
        DVec3::new(-1.0, 1.0, -1.0),
        DVec3::new(-1.0, 1.0, 1.0),
        DVec3::new(1.0, 1.0, 1.0),
    ];
    let right_corners = [
        DVec3::new(1.0, -1.0, -1.0),
        DVec3::new(1.0, 1.0, -1.0),
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(1.0, -1.0, 1.0),
    ];
    let left_corners = [
        DVec3::new(-1.0, 1.0, -1.0),
        DVec3::new(-1.0, -1.0, -1.0),
        DVec3::new(-1.0, -1.0, 1.0),
        DVec3::new(-1.0, 1.0, 1.0),
    ];

    [
        Face {
            label: "TOP",
            normal: DVec3::new(0.0, 0.0, 1.0),
            yaw: FRAC_PI_2,
            pitch: FRAC_PI_2,
            corners: top_corners,
        },
        Face {
            label: "BOTTOM",
            normal: DVec3::new(0.0, 0.0, -1.0),
            yaw: FRAC_PI_2,
            pitch: -FRAC_PI_2,
            corners: bottom_corners,
        },
        Face {
            label: "FRONT",
            normal: DVec3::new(0.0, -1.0, 0.0),
            yaw: FRAC_PI_2,
            pitch: 0.0,
            corners: front_corners,
        },
        Face {
            label: "BACK",
            normal: DVec3::new(0.0, 1.0, 0.0),
            yaw: -FRAC_PI_2,
            pitch: 0.0,
            corners: back_corners,
        },
        Face {
            label: "RIGHT",
            normal: DVec3::new(1.0, 0.0, 0.0),
            yaw: PI,
            pitch: 0.0,
            corners: right_corners,
        },
        Face {
            label: "LEFT",
            normal: DVec3::new(-1.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            corners: left_corners,
        },
    ]
}

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    _response: &mut ShellResponse,
) {
    let center = Pos2::new(
        rect.max.x - GIZMO_PADDING - CUBE_RADIUS * 1.5,
        rect.min.y + GIZMO_PADDING + CUBE_RADIUS * 1.5,
    );

    let yaw = shell.camera.yaw_radians();
    let pitch = shell.camera.pitch_radians();

    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let forward = DVec3::new(cos_yaw * cos_pitch, sin_yaw * cos_pitch, -sin_pitch);
    let right = DVec3::new(sin_yaw, -cos_yaw, 0.0);
    let up = DVec3::new(cos_yaw * sin_pitch, sin_yaw * sin_pitch, cos_pitch);

    let faces = create_faces();
    let mut visible_faces = Vec::new();

    for face in faces {
        let dot = face.normal.dot(forward);
        if dot < 0.0 {
            visible_faces.push((face, dot));
        }
    }

    // Sort so furthest (least negative) is drawn first
    visible_faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let gizmo_rect = Rect::from_center_size(center, Vec2::splat(CUBE_RADIUS * 3.0));
    let interact = ui.interact(gizmo_rect, ui.id().with("nav_gizmo"), egui::Sense::click());

    let pointer_pos = interact.hover_pos();
    let mut hovered_face = None;

    for (face, dot) in visible_faces {
        let mut screen_corners = Vec::with_capacity(4);
        for corner in face.corners {
            let x = corner.dot(right) as f32;
            let y = corner.dot(up) as f32;
            screen_corners.push(center + Vec2::new(x, -y) * CUBE_RADIUS);
        }

        let is_hovered = if let Some(p) = pointer_pos {
            is_point_in_polygon(p, &screen_corners)
        } else {
            false
        };

        if is_hovered {
            hovered_face = Some(face);
        }

        // Depth shading: faces more directly facing the camera are brighter
        let depth_factor = (-dot).clamp(0.0, 1.0) as f32; // 0..1, 1 = facing camera
        let brightness = 0.6 + 0.4 * depth_factor;

        let fill_color = if is_hovered {
            ThemeColors::BG_HOVER
        } else {
            darken_color(ThemeColors::BG_PANEL_ALT_GLASS, brightness)
        };

        let stroke_color = if is_hovered {
            ThemeColors::ACCENT
        } else {
            ThemeColors::SEPARATOR
        };

        ui.painter().add(Shape::convex_polygon(
            screen_corners.clone(),
            fill_color,
            Stroke::new(1.0, stroke_color),
        ));

        let cx = face.normal.dot(right) as f32;
        let cy = face.normal.dot(up) as f32;
        let text_pos = center + Vec2::new(cx, -cy) * CUBE_RADIUS;

        // Smooth label fading: fully visible at dot=-1, fading out at dot=-0.15
        let label_alpha = ((-dot - 0.15) / 0.85).clamp(0.0, 1.0) as f32;
        if label_alpha > 0.01 {
            let base_color = if is_hovered {
                ThemeColors::TEXT
            } else {
                ThemeColors::TEXT_DIM
            };
            let label_color = base_color.gamma_multiply(label_alpha);
            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                face.label,
                egui::FontId::proportional(12.0),
                label_color,
            );
        }
    }

    if interact.clicked() {
        if let Some(face) = hovered_face {
            shell.camera.set_orientation(face.yaw, face.pitch);
        }
    }

    // Projection toggle button
    let toggle_pos = center + Vec2::new(CUBE_RADIUS * 1.5, CUBE_RADIUS * 1.5);
    let toggle_rect = Rect::from_center_size(toggle_pos, Vec2::splat(24.0));
    let toggle_interact = ui.interact(
        toggle_rect,
        ui.id().with("proj_toggle"),
        egui::Sense::click(),
    );

    let is_ortho = shell.camera.projection() == roncad_rendering::Projection::Orthographic;
    let icon = if is_ortho {
        egui_phosphor::regular::SQUARE // representing 2D Orthographic
    } else {
        egui_phosphor::regular::CUBE // representing 3D Perspective
    };

    let toggle_bg = if toggle_interact.hovered() {
        ThemeColors::BG_HOVER
    } else {
        ThemeColors::BG_PANEL_GLASS
    };

    ui.painter().rect_filled(toggle_rect, 4.0, toggle_bg);
    ui.painter().rect_stroke(
        toggle_rect,
        4.0,
        Stroke::new(1.0, ThemeColors::SEPARATOR),
        StrokeKind::Outside,
    );
    ui.painter().text(
        toggle_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(14.0),
        if toggle_interact.hovered() {
            ThemeColors::TEXT
        } else {
            ThemeColors::TEXT_DIM
        },
    );

    if toggle_interact.clicked() {
        shell.camera.toggle_projection();
    }

    // Draw Axis Lines (X, Y, Z)
    let axes_origin = center + Vec2::new(-CUBE_RADIUS * 1.5, CUBE_RADIUS * 1.5);
    let axis_length = 20.0;

    let axes = [
        (DVec3::X, ThemeColors::GRID_AXIS_X, "X"),
        (DVec3::Y, ThemeColors::GRID_AXIS_Y, "Y"),
        (DVec3::Z, ThemeColors::ACCENT, "Z"),
    ];

    // Sort axes so ones pointing towards camera are drawn last
    let mut sorted_axes = axes.to_vec();
    sorted_axes.sort_by(|a, b| {
        let dot_a = a.0.dot(forward);
        let dot_b = b.0.dot(forward);
        dot_b
            .partial_cmp(&dot_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (dir, color, label) in sorted_axes {
        let x = dir.dot(right) as f32;
        let y = dir.dot(up) as f32;
        let end = axes_origin + Vec2::new(x, -y) * axis_length;

        ui.painter()
            .line_segment([axes_origin, end], Stroke::new(2.0, color));

        let len_sq = x * x + y * y;
        if len_sq > 0.01 {
            let label_dir = Vec2::new(x, -y).normalized();
            ui.painter().text(
                end + label_dir * 8.0,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(10.0),
                color,
            );
        }
    }
}

fn is_point_in_polygon(p: Pos2, polygon: &[Pos2]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        if (polygon[i].y > p.y) != (polygon[j].y > p.y)
            && p.x
                < (polygon[j].x - polygon[i].x) * (p.y - polygon[i].y)
                    / (polygon[j].y - polygon[i].y)
                    + polygon[i].x
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn darken_color(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.g() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        (color.b() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        color.a(),
    )
}
