use egui::{Color32, Rect, Stroke};
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{arc_sample_points, HoverTarget, Project, SketchEntity};
use roncad_rendering::Camera2d;

use super::{screen_center, to_pos, tool_overlay, COLOR_SKETCH};
use crate::theme::ThemeColors;

const COLOR_HOVER: Color32 = Color32::from_rgb(0xF5, 0xC2, 0x52);

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
    hovered_target: Option<&HoverTarget>,
) {
    let Some(sketch_id) = project.active_sketch else {
        return;
    };
    let Some(sketch) = project.active_sketch() else {
        return;
    };
    let center = screen_center(rect);

    for (entity_id, entity) in sketch.iter() {
        let selected = selection.contains(&SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity: entity_id,
        });
        let hovered =
            hovered_target.is_some_and(|target| target.matches_sketch_entity(sketch_id, entity_id));
        let color = if selected {
            ThemeColors::ACCENT
        } else if hovered {
            COLOR_HOVER
        } else {
            COLOR_SKETCH
        };
        let stroke_width = if selected && hovered {
            2.8
        } else if selected {
            2.2
        } else if hovered {
            2.0
        } else {
            1.6
        };
        let vertex_width = if selected || hovered { 1.6 } else { 1.0 };
        let point_radius = if selected && hovered {
            4.5
        } else if selected {
            4.0
        } else if hovered {
            3.5
        } else {
            2.5
        };
        let stroke = Stroke::new(stroke_width, color);
        let vertex = Stroke::new(vertex_width, color);
        match entity {
            SketchEntity::Point { p } => {
                let s = to_pos(camera.world_to_screen(*p, center));
                painter.circle_stroke(s, point_radius, vertex);
            }
            SketchEntity::Line { a, b } => {
                let sa = to_pos(camera.world_to_screen(*a, center));
                let sb = to_pos(camera.world_to_screen(*b, center));
                painter.line_segment([sa, sb], stroke);
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                tool_overlay::paint_rect(painter, camera, center, *corner_a, *corner_b, stroke);
            }
            SketchEntity::Circle { center: c, radius } => {
                let sc = to_pos(camera.world_to_screen(*c, center));
                let r_px = (*radius * camera.pixels_per_mm) as f32;
                painter.circle_stroke(sc, r_px, stroke);
            }
            SketchEntity::Arc {
                center: c,
                radius,
                start_angle,
                sweep_angle,
            } => {
                let points: Vec<_> = arc_sample_points(
                    *c,
                    *radius,
                    *start_angle,
                    *sweep_angle,
                    std::f64::consts::PI / 48.0,
                )
                .into_iter()
                .map(|point| to_pos(camera.world_to_screen(point, center)))
                .collect();
                painter.add(egui::Shape::line(points, stroke));
            }
        }
    }
}
