use egui::{Color32, Pos2, Rect, Shape, Stroke};
use glam::DVec3;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::Project;
use roncad_rendering::{extrude_mesh, Camera2d};

use super::{screen_center, to_pos};
use crate::theme::ThemeColors;

const LIGHT_DIR: DVec3 = DVec3::new(-0.42, 0.35, 0.84);

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
    let mut faces = Vec::<ScreenFace>::new();
    let mut edges = Vec::<ScreenEdge>::new();

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
            let mesh = extrude_mesh(feature.profile(), feature.distance_mm());

            for triangle in mesh.triangles {
                let positions = triangle
                    .positions
                    .map(|position| workplane.local_position(position));
                let centroid = (positions[0] + positions[1] + positions[2]) / 3.0;
                let normal = workplane.local_position(triangle.normal)
                    - workplane.local_position(DVec3::ZERO);
                if normal.dot(eye - centroid) <= 0.0 {
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

                let depth = triangle
                    .positions
                    .iter()
                    .map(|position| camera.view_depth(workplane.local_position(*position)))
                    .sum::<f64>()
                    / 3.0;
                let intensity = (0.22 + 0.78 * normal.normalize().dot(light).max(0.0)) as f32;
                faces.push(ScreenFace {
                    points: projected,
                    depth,
                    fill: palette.face_fill(intensity),
                });
            }

            for (start, end) in mesh.outline_edges {
                let start = workplane.local_position(start);
                let end = workplane.local_position(end);
                let (Some(start_screen), Some(end_screen)) = (
                    camera.project_point(start, center),
                    camera.project_point(end, center),
                ) else {
                    continue;
                };
                let depth = (camera.view_depth(start) + camera.view_depth(end)) * 0.5;
                edges.push(ScreenEdge {
                    points: [to_pos(start_screen), to_pos(end_screen)],
                    depth,
                    stroke: palette.edge_stroke,
                });
            }
        }
    }

    faces.sort_by(|lhs, rhs| rhs.depth.total_cmp(&lhs.depth));
    for face in faces {
        painter.add(Shape::convex_polygon(
            face.points.to_vec(),
            face.fill,
            Stroke::NONE,
        ));
    }

    edges.sort_by(|lhs, rhs| rhs.depth.total_cmp(&lhs.depth));
    for edge in edges {
        painter.line_segment(edge.points, edge.stroke);
    }
}

struct ScreenFace {
    points: [Pos2; 3],
    depth: f64,
    fill: Color32,
}

struct ScreenEdge {
    points: [Pos2; 2],
    depth: f64,
    stroke: Stroke,
}

struct BodyPalette {
    diffuse_rgb: [u8; 3],
    edge_stroke: Stroke,
}

impl BodyPalette {
    fn for_selection(selected: bool) -> Self {
        if selected {
            Self {
                diffuse_rgb: [0x56, 0xA6, 0xF0],
                edge_stroke: Stroke::new(1.2, ThemeColors::ACCENT.gamma_multiply(0.9)),
            }
        } else {
            Self {
                diffuse_rgb: [0x8D, 0x98, 0xA8],
                edge_stroke: Stroke::new(0.9, ThemeColors::SEPARATOR.gamma_multiply(0.9)),
            }
        }
    }

    fn face_fill(&self, intensity: f32) -> Color32 {
        let ambient = 0.35;
        let scaled = ambient + (1.0 - ambient) * intensity.clamp(0.0, 1.0);
        Color32::from_rgb(
            scale_channel(self.diffuse_rgb[0], scaled),
            scale_channel(self.diffuse_rgb[1], scaled),
            scale_channel(self.diffuse_rgb[2], scaled),
        )
    }
}

fn scale_channel(channel: u8, scale: f32) -> u8 {
    ((channel as f32 * scale).round()).clamp(0.0, 255.0) as u8
}
