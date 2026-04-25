use egui::{Color32, Pos2, Rect, Shape, Stroke};
use roncad_core::constraint::EntityPoint;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{
    arc_sample_points, resolve_entity_point, ConstraintDiagnosticKind, HoverTarget, Project,
    SketchEntity, SolveReport,
};
use roncad_rendering::Camera2d;

use super::{project_workplane_point, screen_center, tool_overlay, COLOR_SKETCH};
use crate::shell::SelectionMoveDrag;
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
        let halo = emphasis_halo(selected, hovered, highlighted, problem, color, stroke_width);
        let emphasize_handles = selected || hovered || highlighted;
        match entity {
            SketchEntity::Point { p } => {
                if let Some(s) = project_workplane_point(camera, center, workplane, *p) {
                    if let Some(halo) = halo {
                        painter.circle_filled(s, point_radius + 2.8, halo.color);
                    }
                    let vertex_selected =
                        vertex_selected(selection, sketch_id, EntityPoint::Point(entity_id));
                    let vertex_hovered =
                        vertex_hovered(hovered_target, sketch_id, EntityPoint::Point(entity_id));
                    paint_selectable_handle(
                        painter,
                        s,
                        color,
                        vertex_selected,
                        vertex_hovered || hovered,
                    );
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
                    let start_selected =
                        vertex_selected(selection, sketch_id, EntityPoint::Start(entity_id));
                    let end_selected =
                        vertex_selected(selection, sketch_id, EntityPoint::End(entity_id));
                    let start_hovered =
                        vertex_hovered(hovered_target, sketch_id, EntityPoint::Start(entity_id));
                    let end_hovered =
                        vertex_hovered(hovered_target, sketch_id, EntityPoint::End(entity_id));
                    if emphasize_handles
                        || start_selected
                        || end_selected
                        || start_hovered
                        || end_hovered
                    {
                        paint_selectable_handle(
                            painter,
                            sa,
                            color,
                            start_selected,
                            start_hovered || hovered,
                        );
                        paint_selectable_handle(
                            painter,
                            sb,
                            color,
                            end_selected,
                            end_hovered || hovered,
                        );
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
                let corners = [
                    (EntityPoint::CornerA(entity_id), *corner_a),
                    (
                        EntityPoint::CornerB(entity_id),
                        glam::dvec2(corner_b.x, corner_a.y),
                    ),
                    (EntityPoint::CornerC(entity_id), *corner_b),
                    (
                        EntityPoint::CornerD(entity_id),
                        glam::dvec2(corner_a.x, corner_b.y),
                    ),
                ];
                for (handle, corner) in corners {
                    let corner_selected = vertex_selected(selection, sketch_id, handle);
                    let corner_hovered = vertex_hovered(hovered_target, sketch_id, handle);
                    if emphasize_handles || corner_selected || corner_hovered {
                        if let Some(pos) =
                            project_workplane_point(camera, center, workplane, corner)
                        {
                            paint_selectable_handle(
                                painter,
                                pos,
                                color,
                                corner_selected,
                                corner_hovered || hovered,
                            );
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
                let center_selected =
                    vertex_selected(selection, sketch_id, EntityPoint::Center(entity_id));
                let center_hovered =
                    vertex_hovered(hovered_target, sketch_id, EntityPoint::Center(entity_id));
                if emphasize_handles || center_selected || center_hovered {
                    if let Some(center_pos) = project_workplane_point(camera, center, workplane, *c)
                    {
                        paint_selectable_handle(
                            painter,
                            center_pos,
                            color,
                            center_selected,
                            center_hovered || hovered,
                        );
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
                let start = *c + glam::DVec2::new(start_angle.cos(), start_angle.sin()) * *radius;
                let end = *c
                    + glam::DVec2::new(
                        (*start_angle + *sweep_angle).cos(),
                        (*start_angle + *sweep_angle).sin(),
                    ) * *radius;
                let handles = [
                    (EntityPoint::Start(entity_id), start),
                    (EntityPoint::End(entity_id), end),
                    (EntityPoint::Center(entity_id), *c),
                ];
                for (handle, point) in handles {
                    let handle_selected = vertex_selected(selection, sketch_id, handle);
                    let handle_hovered = vertex_hovered(hovered_target, sketch_id, handle);
                    if emphasize_handles || handle_selected || handle_hovered {
                        if let Some(pos) = project_workplane_point(camera, center, workplane, point)
                        {
                            paint_selectable_handle(
                                painter,
                                pos,
                                color,
                                handle_selected,
                                handle_hovered || hovered,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn paint_move_preview(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    drag: Option<&SelectionMoveDrag>,
) {
    let Some(drag) = drag else {
        return;
    };
    let delta = drag.delta();
    if delta.length_squared() <= f64::EPSILON {
        return;
    }
    let Some(sketch) = project.sketches.get(drag.sketch) else {
        return;
    };
    let Some(workplane) = project.sketch_workplane(drag.sketch) else {
        return;
    };
    let center = screen_center(rect);
    let stroke = Stroke::new(2.0, with_alpha(ThemeColors::ACCENT, 170));
    let handle_color = ThemeColors::ACCENT;

    for entity_id in &drag.entities {
        let Some(entity) = sketch.entities.get(*entity_id) else {
            continue;
        };
        paint_entity_offset(painter, camera, center, workplane, entity, delta, stroke);
    }

    for point in &drag.vertices {
        let Some(entity) = sketch.entities.get(point.entity()) else {
            continue;
        };
        let Some(origin) = resolve_entity_point(*point, entity) else {
            continue;
        };
        if let (Some(a), Some(b)) = (
            project_workplane_point(camera, center, workplane, origin),
            project_workplane_point(camera, center, workplane, origin + delta),
        ) {
            painter.line_segment([a, b], Stroke::new(1.2, with_alpha(handle_color, 120)));
            paint_selectable_handle(painter, b, handle_color, true, true);
        }
    }
}

fn paint_entity_offset(
    painter: &egui::Painter,
    camera: &Camera2d,
    screen_center: glam::DVec2,
    workplane: &roncad_geometry::Workplane,
    entity: &SketchEntity,
    delta: glam::DVec2,
    stroke: Stroke,
) {
    match entity {
        SketchEntity::Point { p } => {
            if let Some(pos) = project_workplane_point(camera, screen_center, workplane, *p + delta)
            {
                paint_selectable_handle(painter, pos, stroke.color, true, true);
            }
        }
        SketchEntity::Line { a, b } => {
            if let (Some(sa), Some(sb)) = (
                project_workplane_point(camera, screen_center, workplane, *a + delta),
                project_workplane_point(camera, screen_center, workplane, *b + delta),
            ) {
                painter.line_segment([sa, sb], stroke);
            }
        }
        SketchEntity::Rectangle { corner_a, corner_b } => {
            tool_overlay::paint_rect(
                painter,
                camera,
                screen_center,
                workplane,
                *corner_a + delta,
                *corner_b + delta,
                stroke,
            );
        }
        SketchEntity::Circle { center, radius } => {
            let points: Vec<_> = arc_sample_points(
                *center + delta,
                *radius,
                0.0,
                std::f64::consts::TAU,
                std::f64::consts::PI / 32.0,
            )
            .into_iter()
            .filter_map(|point| project_workplane_point(camera, screen_center, workplane, point))
            .collect();
            painter.add(Shape::closed_line(points, stroke));
        }
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            let points: Vec<_> = arc_sample_points(
                *center + delta,
                *radius,
                *start_angle,
                *sweep_angle,
                std::f64::consts::PI / 48.0,
            )
            .into_iter()
            .filter_map(|point| project_workplane_point(camera, screen_center, workplane, point))
            .collect();
            painter.add(Shape::line(points, stroke));
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

fn paint_selectable_handle(
    painter: &egui::Painter,
    pos: Pos2,
    entity_color: Color32,
    selected: bool,
    hovered: bool,
) {
    let color = if selected {
        ThemeColors::ACCENT
    } else {
        entity_color
    };
    let radius = if selected {
        5.2
    } else if hovered {
        4.2
    } else {
        3.5
    };
    painter.circle_filled(
        pos,
        radius + 1.8,
        with_alpha(color, if selected { 72 } else { 42 }),
    );
    painter.circle_filled(pos, radius * 0.54, color);
    painter.circle_stroke(
        pos,
        radius,
        Stroke::new(
            if selected { 1.6 } else { 1.0 },
            with_alpha(color, if selected { 190 } else { 96 }),
        ),
    );
}

fn vertex_selected(
    selection: &Selection,
    sketch: roncad_core::ids::SketchId,
    point: EntityPoint,
) -> bool {
    selection.contains(&SelectionItem::SketchVertex { sketch, point })
}

fn vertex_hovered(
    hovered_target: Option<&HoverTarget>,
    sketch: roncad_core::ids::SketchId,
    point: EntityPoint,
) -> bool {
    hovered_target.is_some_and(|target| target.matches_sketch_vertex(sketch, point))
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
