//! Painter-based rendering of extruded bodies in the viewport.
//! Uses per-vertex normals from the mesh for smooth shading on curved surfaces,
//! and an ambient + diffuse + specular lighting model for solid appearance.

use egui::{Color32, Pos2, Rect, Shape, Stroke};
use glam::DVec3;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::Project;
use roncad_rendering::{extrude_mesh, revolve_mesh, Camera2d, EdgeKind};

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

const LIGHT_DIR: DVec3 = DVec3::new(-0.42, 0.35, 0.84);

// Lighting parameters
const AMBIENT: f32 = 0.18;
const DIFFUSE_WEIGHT: f32 = 0.62;
const SPECULAR_WEIGHT: f32 = 0.20;
const SPECULAR_POWER: f32 = 32.0;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
) {
    let center = screen_center(rect);
    let eye = camera.eye_mm();
    let light = LIGHT_DIR.normalize();
    let mut items = Vec::<ScreenItem>::new();

    for (body_id, _) in project.bodies.iter() {
        let selected = selection.contains(&SelectionItem::Body(body_id));
        let palette = BodyPalette::for_selection(selected);

        for (_, feature) in project.body_features(body_id) {
            let Some(workplane) = feature
                .source_sketch()
                .and_then(|sketch_id| project.sketch_workplane(sketch_id))
                .or_else(|| project.workplanes.iter().next().map(|(_, plane)| plane))
            else {
                continue;
            };
            let mesh = match feature {
                roncad_geometry::Feature::Extrude(f) => extrude_mesh(&f.profile, f.distance_mm),
                roncad_geometry::Feature::Revolve(f) => revolve_mesh(&f.profile, f.axis_origin, f.axis_dir, f.angle_rad),
            };

            for triangle in mesh.triangles {
                let positions = triangle
                    .vertices
                    .each_ref()
                    .map(|v| workplane.local_position(v.position));
                let normals = triangle.vertices.each_ref().map(|v| {
                    (workplane.local_position(v.normal) - workplane.local_position(DVec3::ZERO))
                        .normalize_or_zero()
                });
                let centroid = (positions[0] + positions[1] + positions[2]) / 3.0;
                let avg_normal = (normals[0] + normals[1] + normals[2]).normalize_or_zero();

                // Back-face cull using geometric normal
                if avg_normal.dot(eye - centroid) <= 0.0 {
                    continue;
                }

                let mut projected = [Pos2::ZERO; 3];
                let mut skip = false;
                for (index, position) in positions.iter().enumerate() {
                    let Some(screen) = camera.project_point(*position, center) else {
                        skip = true;
                        break;
                    };
                    projected[index] = to_pos(screen);
                }
                if skip {
                    continue;
                }

                let depth = positions
                    .iter()
                    .map(|position| camera.view_depth(*position))
                    .sum::<f64>()
                    / 3.0;

                // Per-vertex lighting, averaged for fill color
                let view_dir = (eye - centroid).normalize_or_zero();
                let fill = palette.lit_face_color(&normals, &light, &view_dir);

                items.push(ScreenItem::Face {
                    points: projected,
                    depth,
                    fill,
                });
            }

            for edge in mesh.edges {
                let start = workplane.local_position(edge.start);
                let end = workplane.local_position(edge.end);
                let (Some(start_screen), Some(end_screen)) = (
                    camera.project_point(start, center),
                    camera.project_point(end, center),
                ) else {
                    continue;
                };
                let depth = (camera.view_depth(start) + camera.view_depth(end)) * 0.5;
                items.push(ScreenItem::Edge {
                    points: [to_pos(start_screen), to_pos(end_screen)],
                    depth: depth - 0.1, // Bias edge towards camera to draw on top of coplanar faces
                    stroke: palette.edge_stroke(edge.kind),
                });
            }
        }
    }

    items.sort_by(|lhs, rhs| rhs.depth().total_cmp(&lhs.depth()));
    for item in items {
        match item {
            ScreenItem::Face { points, fill, .. } => {
                painter.add(Shape::convex_polygon(points.to_vec(), fill, Stroke::NONE));
            }
            ScreenItem::Edge { points, stroke, .. } => {
                painter.line_segment(points, stroke);
            }
        }
    }
}

enum ScreenItem {
    Face {
        points: [Pos2; 3],
        depth: f64,
        fill: Color32,
    },
    Edge {
        points: [Pos2; 2],
        depth: f64,
        stroke: Stroke,
    },
}

impl ScreenItem {
    fn depth(&self) -> f64 {
        match self {
            Self::Face { depth, .. } => *depth,
            Self::Edge { depth, .. } => *depth,
        }
    }
}

struct BodyPalette {
    diffuse_rgb: [f32; 3],
    selected: bool,
}

impl BodyPalette {
    fn for_selection(selected: bool) -> Self {
        if selected {
            Self {
                diffuse_rgb: [0x56 as f32 / 255.0, 0xA6 as f32 / 255.0, 0xF0 as f32 / 255.0],
                selected: true,
            }
        } else {
            Self {
                diffuse_rgb: [0x8D as f32 / 255.0, 0x98 as f32 / 255.0, 0xA8 as f32 / 255.0],
                selected: false,
            }
        }
    }

    fn lit_face_color(&self, normals: &[DVec3; 3], light: &DVec3, view_dir: &DVec3) -> Color32 {
        // Compute per-vertex intensity then average
        let mut total_r = 0.0_f32;
        let mut total_g = 0.0_f32;
        let mut total_b = 0.0_f32;

        for normal in normals {
            let n = if normal.length_squared() > 1e-12 {
                normal.normalize()
            } else {
                DVec3::Z
            };

            let diffuse = n.dot(*light).max(0.0) as f32;

            // Blinn-Phong specular: halfway vector
            let halfway = (*light + *view_dir).normalize_or_zero();
            let spec = n.dot(halfway).max(0.0).powf(SPECULAR_POWER as f64) as f32;

            let intensity = AMBIENT + DIFFUSE_WEIGHT * diffuse + SPECULAR_WEIGHT * spec;

            total_r += (self.diffuse_rgb[0] * intensity).min(1.0);
            total_g += (self.diffuse_rgb[1] * intensity).min(1.0);
            total_b += (self.diffuse_rgb[2] * intensity).min(1.0);
        }

        Color32::from_rgb(
            ((total_r / 3.0) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((total_g / 3.0) * 255.0).round().clamp(0.0, 255.0) as u8,
            ((total_b / 3.0) * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    fn edge_stroke(&self, kind: EdgeKind) -> Stroke {
        match kind {
            EdgeKind::Crease => {
                if self.selected {
                    Stroke::new(1.4, ThemeColors::ACCENT.gamma_multiply(0.9))
                } else {
                    Stroke::new(1.0, ThemeColors::SEPARATOR.gamma_multiply(0.9))
                }
            }
            EdgeKind::Border => {
                if self.selected {
                    Stroke::new(0.8, ThemeColors::ACCENT.gamma_multiply(0.6))
                } else {
                    Stroke::new(0.6, ThemeColors::SEPARATOR.gamma_multiply(0.5))
                }
            }
        }
    }
}
