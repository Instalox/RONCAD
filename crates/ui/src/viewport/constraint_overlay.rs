//! Paints small glyphs indicating which constraints apply to sketch
//! entities. Keeps the same muted style as dimension annotations so the
//! sketch stays legible.

use egui::{Align2, Color32, FontId, Rect};
use glam::DVec2;
use roncad_core::constraint::{Constraint, EntityPoint};
use roncad_core::ids::{ConstraintId, SketchEntityId, SketchId};
use roncad_geometry::{
    resolve_entity_point, ConstraintDiagnosticKind, Project, SketchEntity, SolveReport, Workplane,
};
use roncad_rendering::Camera2d;

use super::{project_workplane_point, screen_center};
use crate::theme::ThemeColors;

const GLYPH_COLOR: Color32 = Color32::from_rgb(0x9F, 0xBF, 0xDF);
const GLYPH_SHADOW: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
const GLYPH_OFFSET_PX: f32 = 10.0;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    report: Option<&SolveReport>,
    highlighted_constraint: Option<(SketchId, ConstraintId)>,
) {
    let Some(sketch) = project.active_sketch() else {
        return;
    };
    let Some(workplane) = project.active_workplane() else {
        return;
    };
    let center = screen_center(rect);
    let font = FontId::monospace(10.0);

    for (constraint_id, constraint) in sketch.iter_constraints() {
        paint_constraint(
            painter,
            camera,
            center,
            workplane,
            sketch,
            constraint_id,
            constraint,
            report,
            &font,
            highlighted_constraint
                .filter(|(sketch_id, _)| Some(*sketch_id) == project.active_sketch)
                .map(|(_, id)| id),
        );
    }
}

fn paint_constraint(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    sketch: &roncad_geometry::Sketch,
    constraint_id: ConstraintId,
    constraint: &Constraint,
    report: Option<&SolveReport>,
    font: &FontId,
    highlighted_constraint: Option<ConstraintId>,
) {
    let color = if highlighted_constraint == Some(constraint_id) {
        ThemeColors::ACCENT_GREEN
    } else {
        diagnostic_color(report, constraint_id)
    };
    match *constraint {
        Constraint::Horizontal { entity } => {
            if let Some(point) = line_midpoint(sketch, entity) {
                glyph_at(
                    painter, camera, center, workplane, point, "H", 1, font, color,
                );
            }
        }
        Constraint::Vertical { entity } => {
            if let Some(point) = line_midpoint(sketch, entity) {
                glyph_at(
                    painter, camera, center, workplane, point, "V", 1, font, color,
                );
            }
        }
        Constraint::Coincident { a, b } => {
            if let Some(point) = resolve_handle(sketch, a).or_else(|| resolve_handle(sketch, b)) {
                dot_at(painter, camera, center, workplane, point, color);
            }
        }
        Constraint::FixPoint { point, .. } => {
            if let Some(world) = resolve_handle(sketch, point) {
                glyph_at(
                    painter, camera, center, workplane, world, "Fx", 0, font, color,
                );
            }
        }
        Constraint::PointOnEntity { point, .. } => {
            if let Some(world) = resolve_handle(sketch, point) {
                ring_at(painter, camera, center, workplane, world, color);
            }
        }
        Constraint::Parallel { a, b } => {
            for (slot, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(
                        painter,
                        camera,
                        center,
                        workplane,
                        point,
                        "||",
                        slot as i32,
                        font,
                        color,
                    );
                }
            }
        }
        Constraint::Perpendicular { a, b } => {
            for (slot, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(
                        painter,
                        camera,
                        center,
                        workplane,
                        point,
                        "_|_",
                        slot as i32,
                        font,
                        color,
                    );
                }
            }
        }
        Constraint::Tangent { line, curve } => {
            if let Some(point) = line_midpoint(sketch, line) {
                glyph_at(
                    painter, camera, center, workplane, point, "T", 0, font, color,
                );
            }
            if let Some(point) = entity_center(sketch, curve) {
                glyph_at(
                    painter, camera, center, workplane, point, "T", 1, font, color,
                );
            }
        }
        Constraint::EqualLength { a, b } => {
            for (slot, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(
                        painter,
                        camera,
                        center,
                        workplane,
                        point,
                        "=",
                        slot as i32,
                        font,
                        color,
                    );
                }
            }
        }
        Constraint::EqualRadius { a, b } => {
            for (slot, curve) in [a, b].iter().enumerate() {
                if let Some(point) = entity_center(sketch, *curve) {
                    glyph_at(
                        painter,
                        camera,
                        center,
                        workplane,
                        point,
                        "R=",
                        slot as i32,
                        font,
                        color,
                    );
                }
            }
        }
    }
}

fn glyph_at(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    world: DVec2,
    label: &str,
    slot: i32,
    font: &FontId,
    color: Color32,
) {
    let Some(screen) = project_workplane_point(camera, center, workplane, world) else {
        return;
    };
    let offset = egui::vec2(GLYPH_OFFSET_PX + slot as f32 * 12.0, -GLYPH_OFFSET_PX);
    let anchor = screen + offset;
    painter.text(
        anchor + egui::vec2(1.0, 1.0),
        Align2::LEFT_BOTTOM,
        label,
        font.clone(),
        GLYPH_SHADOW,
    );
    painter.text(anchor, Align2::LEFT_BOTTOM, label, font.clone(), color);
}

fn dot_at(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    world: DVec2,
    color: Color32,
) {
    let Some(screen) = project_workplane_point(camera, center, workplane, world) else {
        return;
    };
    painter.circle_filled(screen, 3.0, color);
}

fn ring_at(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    world: DVec2,
    color: Color32,
) {
    let Some(screen) = project_workplane_point(camera, center, workplane, world) else {
        return;
    };
    painter.circle_stroke(screen, 3.5, egui::Stroke::new(1.2, color));
}

fn line_midpoint(sketch: &roncad_geometry::Sketch, id: SketchEntityId) -> Option<DVec2> {
    match sketch.entities.get(id)? {
        SketchEntity::Line { a, b } => Some((*a + *b) * 0.5),
        _ => None,
    }
}

fn entity_center(sketch: &roncad_geometry::Sketch, id: SketchEntityId) -> Option<DVec2> {
    match sketch.entities.get(id)? {
        SketchEntity::Circle { center, .. } | SketchEntity::Arc { center, .. } => Some(*center),
        _ => None,
    }
}

fn resolve_handle(sketch: &roncad_geometry::Sketch, handle: EntityPoint) -> Option<DVec2> {
    let entity = sketch.entities.get(handle.entity())?;
    resolve_entity_point(handle, entity)
}

fn diagnostic_color(report: Option<&SolveReport>, constraint_id: ConstraintId) -> Color32 {
    let Some(report) = report else {
        return GLYPH_COLOR;
    };
    match report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == constraint_id)
        .map(|diagnostic| diagnostic.kind)
    {
        Some(ConstraintDiagnosticKind::Unsatisfied) => ThemeColors::ACCENT_AMBER,
        Some(ConstraintDiagnosticKind::Failed) => ThemeColors::ACCENT_RED,
        None => GLYPH_COLOR,
    }
}
