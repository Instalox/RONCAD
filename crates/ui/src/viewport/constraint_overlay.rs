//! Paints small glyphs indicating which constraints apply to sketch
//! entities. Keeps the same muted style as dimension annotations so the
//! sketch stays legible.

use egui::{Align2, Color32, FontId, Rect};
use glam::DVec2;
use roncad_core::constraint::{Constraint, EntityPoint};
use roncad_core::ids::SketchEntityId;
use roncad_geometry::{resolve_entity_point, Project, SketchEntity, Workplane};
use roncad_rendering::Camera2d;

use super::{project_workplane_point, screen_center};

const GLYPH_COLOR: Color32 = Color32::from_rgb(0x9F, 0xBF, 0xDF);
const GLYPH_SHADOW: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
const GLYPH_OFFSET_PX: f32 = 10.0;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
) {
    let Some(sketch) = project.active_sketch() else {
        return;
    };
    let Some(workplane) = project.active_workplane() else {
        return;
    };
    let center = screen_center(rect);
    let font = FontId::monospace(10.0);

    for (_, constraint) in sketch.iter_constraints() {
        paint_constraint(painter, camera, center, workplane, sketch, constraint, &font);
    }
}

fn paint_constraint(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    sketch: &roncad_geometry::Sketch,
    constraint: &Constraint,
    font: &FontId,
) {
    match *constraint {
        Constraint::Horizontal { entity } => {
            if let Some(point) = line_midpoint(sketch, entity) {
                glyph_at(painter, camera, center, workplane, point, "H", 1, font);
            }
        }
        Constraint::Vertical { entity } => {
            if let Some(point) = line_midpoint(sketch, entity) {
                glyph_at(painter, camera, center, workplane, point, "V", 1, font);
            }
        }
        Constraint::Coincident { a, b } => {
            if let Some(point) = resolve_handle(sketch, a).or_else(|| resolve_handle(sketch, b)) {
                dot_at(painter, camera, center, workplane, point);
            }
        }
        Constraint::PointOnEntity { point, .. } => {
            if let Some(world) = resolve_handle(sketch, point) {
                ring_at(painter, camera, center, workplane, world);
            }
        }
        Constraint::Parallel { a, b } => {
            for (i, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(painter, camera, center, workplane, point, "∥", i as i32, font);
                }
            }
        }
        Constraint::Perpendicular { a, b } => {
            for (i, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(painter, camera, center, workplane, point, "⊥", i as i32, font);
                }
            }
        }
        Constraint::Tangent { line, curve } => {
            if let Some(point) = line_midpoint(sketch, line) {
                glyph_at(painter, camera, center, workplane, point, "T", 0, font);
            }
            if let Some(point) = entity_center(sketch, curve) {
                glyph_at(painter, camera, center, workplane, point, "T", 1, font);
            }
        }
        Constraint::EqualLength { a, b } => {
            for (i, line) in [a, b].iter().enumerate() {
                if let Some(point) = line_midpoint(sketch, *line) {
                    glyph_at(painter, camera, center, workplane, point, "=", i as i32, font);
                }
            }
        }
        Constraint::EqualRadius { a, b } => {
            for (i, curve) in [a, b].iter().enumerate() {
                if let Some(point) = entity_center(sketch, *curve) {
                    glyph_at(painter, camera, center, workplane, point, "R=", i as i32, font);
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
    painter.text(anchor, Align2::LEFT_BOTTOM, label, font.clone(), GLYPH_COLOR);
}

fn dot_at(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    world: DVec2,
) {
    let Some(screen) = project_workplane_point(camera, center, workplane, world) else {
        return;
    };
    painter.circle_filled(screen, 3.0, GLYPH_COLOR);
}

fn ring_at(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: DVec2,
    workplane: &Workplane,
    world: DVec2,
) {
    let Some(screen) = project_workplane_point(camera, center, workplane, world) else {
        return;
    };
    painter.circle_stroke(screen, 3.5, egui::Stroke::new(1.2, GLYPH_COLOR));
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
