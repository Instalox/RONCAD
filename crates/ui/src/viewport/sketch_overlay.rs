use egui::{Color32, Rect, Stroke};
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{
    arc_sample_points, ConstraintDiagnosticKind, HoverTarget, Project, SketchEntity, SolveReport,
};
use roncad_rendering::Camera2d;

use super::{project_workplane_point, screen_center, tool_overlay, COLOR_SKETCH};
use crate::theme::ThemeColors;

const COLOR_HOVER: Color32 = Color32::from_rgb(0xF5, 0xC2, 0x52);

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
    report: Option<&SolveReport>,
    hovered_target: Option<&HoverTarget>,
) {
    let Some(sketch_id) = project.active_sketch else {
        return;
    };
    let Some(sketch) = project.active_sketch() else {
        return;
    };
    let Some(workplane) = project.active_workplane() else {
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
        let problem = problem_entity_kind(report, entity_id);
        let color = if selected {
            ThemeColors::ACCENT
        } else if hovered {
            COLOR_HOVER
        } else if problem == Some(ConstraintDiagnosticKind::Failed) {
            ThemeColors::ACCENT_RED
        } else if problem == Some(ConstraintDiagnosticKind::Unsatisfied) {
            ThemeColors::ACCENT_AMBER
        } else {
            COLOR_SKETCH
        };
        let stroke_width = if selected && hovered {
            2.8
        } else if selected {
            2.2
        } else if hovered {
            2.0
        } else if problem.is_some() {
            2.1
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
                if let Some(s) = project_workplane_point(camera, center, workplane, *p) {
                    painter.circle_stroke(s, point_radius, vertex);
                }
            }
            SketchEntity::Line { a, b } => {
                if let (Some(sa), Some(sb)) = (
                    project_workplane_point(camera, center, workplane, *a),
                    project_workplane_point(camera, center, workplane, *b),
                ) {
                    painter.line_segment([sa, sb], stroke);
                }
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                tool_overlay::paint_rect(
                    painter, camera, center, workplane, *corner_a, *corner_b, stroke,
                );
            }
            SketchEntity::Circle { center: c, radius } => {
                let points: Vec<_> = arc_sample_points(
                    *c,
                    *radius,
                    0.0,
                    std::f64::consts::TAU,
                    std::f64::consts::PI / 32.0,
                )
                .into_iter()
                .filter_map(|point| project_workplane_point(camera, center, workplane, point))
                .collect();
                painter.add(egui::Shape::closed_line(points, stroke));
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
                .filter_map(|point| project_workplane_point(camera, center, workplane, point))
                .collect();
                painter.add(egui::Shape::line(points, stroke));
            }
        }
    }
}

fn problem_entity_kind(
    report: Option<&SolveReport>,
    entity_id: roncad_core::ids::SketchEntityId,
) -> Option<ConstraintDiagnosticKind> {
    let report = report?;
    let mut kind = None;
    for diagnostic in &report.diagnostics {
        if diagnostic.referenced_entities.contains(&entity_id) {
            kind = Some(match (kind, diagnostic.kind) {
                (Some(ConstraintDiagnosticKind::Failed), _) => ConstraintDiagnosticKind::Failed,
                (_, ConstraintDiagnosticKind::Failed) => ConstraintDiagnosticKind::Failed,
                _ => ConstraintDiagnosticKind::Unsatisfied,
            });
        }
    }
    kind
}
