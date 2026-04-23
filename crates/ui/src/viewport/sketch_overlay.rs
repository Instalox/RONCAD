use egui::{Color32, Pos2, Rect, Shape, Stroke};
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
    highlighted_entities: &[(roncad_core::ids::SketchId, roncad_core::ids::SketchEntityId)],
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
        let highlighted = highlighted_entities
            .iter()
            .any(|(sketch, entity)| *sketch == sketch_id && *entity == entity_id);
        let problem = problem_entity_kind(report, entity_id);
        let color = if selected {
            ThemeColors::ACCENT
        } else if hovered {
            COLOR_HOVER
        } else if highlighted {
            ThemeColors::ACCENT_GREEN
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
        } else if highlighted {
            2.2
        } else if problem.is_some() {
            2.1
        } else {
            1.6
        };
        let vertex_width = if selected || hovered || highlighted {
            1.6
        } else {
            1.0
        };
        let point_radius = if selected && hovered {
            4.5
        } else if selected {
            4.0
        } else if hovered {
            3.5
        } else if highlighted {
            3.8
        } else {
            2.5
        };
        let stroke = Stroke::new(stroke_width, color);
        let vertex = Stroke::new(vertex_width, color);
        let halo = emphasis_halo(selected, hovered, highlighted, problem, color, stroke_width);
        let emphasize_handles = selected || hovered || highlighted;
        match entity {
            SketchEntity::Point { p } => {
                if let Some(s) = project_workplane_point(camera, center, workplane, *p) {
                    if let Some(halo) = halo {
                        painter.circle_filled(s, point_radius + 2.8, halo.color);
                    }
                    painter.circle_stroke(s, point_radius, vertex);
                    painter.circle_filled(s, point_radius * 0.42, color);
                }
            }
            SketchEntity::Line { a, b } => {
                if let (Some(sa), Some(sb)) = (
                    project_workplane_point(camera, center, workplane, *a),
                    project_workplane_point(camera, center, workplane, *b),
                ) {
                    if let Some(halo) = halo {
                        painter.line_segment([sa, sb], halo);
                    }
                    painter.line_segment([sa, sb], stroke);
                    if emphasize_handles {
                        paint_handle(painter, sa, color, hovered);
                        paint_handle(painter, sb, color, hovered);
                    }
                }
            }
            SketchEntity::Rectangle { corner_a, corner_b } => {
                if let Some(halo) = halo {
                    tool_overlay::paint_rect(
                        painter, camera, center, workplane, *corner_a, *corner_b, halo,
                    );
                }
                tool_overlay::paint_rect(
                    painter, camera, center, workplane, *corner_a, *corner_b, stroke,
                );
                if emphasize_handles {
                    let corners = [
                        *corner_a,
                        glam::dvec2(corner_b.x, corner_a.y),
                        *corner_b,
                        glam::dvec2(corner_a.x, corner_b.y),
                    ];
                    for corner in corners {
                        if let Some(pos) =
                            project_workplane_point(camera, center, workplane, corner)
                        {
                            paint_handle(painter, pos, color, hovered);
                        }
                    }
                }
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
                if let Some(halo) = halo {
                    painter.add(Shape::closed_line(points.clone(), halo));
                }
                painter.add(Shape::closed_line(points, stroke));
                if emphasize_handles {
                    if let Some(center_pos) = project_workplane_point(camera, center, workplane, *c)
                    {
                        paint_handle(painter, center_pos, color, hovered);
                    }
                }
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
                if let Some(halo) = halo {
                    painter.add(Shape::line(points.clone(), halo));
                }
                painter.add(Shape::line(points, stroke));
                if emphasize_handles {
                    let start =
                        *c + glam::DVec2::new(start_angle.cos(), start_angle.sin()) * *radius;
                    let end = *c
                        + glam::DVec2::new(
                            (*start_angle + *sweep_angle).cos(),
                            (*start_angle + *sweep_angle).sin(),
                        ) * *radius;
                    for point in [start, end] {
                        if let Some(pos) = project_workplane_point(camera, center, workplane, point)
                        {
                            paint_handle(painter, pos, color, hovered);
                        }
                    }
                }
            }
        }
    }
}

fn emphasis_halo(
    selected: bool,
    hovered: bool,
    highlighted: bool,
    problem: Option<ConstraintDiagnosticKind>,
    color: Color32,
    stroke_width: f32,
) -> Option<Stroke> {
    let alpha = if selected && hovered {
        96
    } else if selected {
        78
    } else if hovered {
        70
    } else if highlighted {
        68
    } else if problem.is_some() {
        60
    } else {
        0
    };
    (alpha > 0).then(|| Stroke::new(stroke_width + 3.0, with_alpha(color, alpha)))
}

fn paint_handle(painter: &egui::Painter, pos: Pos2, color: Color32, hovered: bool) {
    painter.circle_filled(pos, if hovered { 4.0 } else { 3.5 }, with_alpha(color, 42));
    painter.circle_filled(pos, if hovered { 2.4 } else { 2.1 }, color);
    painter.circle_stroke(
        pos,
        if hovered { 4.0 } else { 3.5 },
        Stroke::new(1.0, with_alpha(color, 96)),
    );
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_premultiplied(r, g, b, alpha)
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
