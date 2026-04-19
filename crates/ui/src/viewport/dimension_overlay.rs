//! Paints persistent sketch dimensions and read-only derived annotations for
//! the current selection.

use egui::{Color32, FontId, Rect, Stroke};
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;

use super::{screen_center, to_pos};
use crate::dimensions::{self, DimensionAnnotation};

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
) {
    let center = screen_center(rect);
    let font = FontId::monospace(11.0);

    for annotation in dimensions::active_sketch_dimension_annotations(project) {
        paint_persistent_annotation(painter, camera, center, &annotation, &font);
    }

    for entity in dimensions::selected_entity_dimensions(project, selection) {
        for annotation in entity.annotations {
            paint_annotation(painter, camera, center, &annotation, &font);
        }
    }
}

fn paint_annotation(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    annotation: &DimensionAnnotation,
    font: &FontId,
) {
    let shadow_color = Color32::from_rgba_unmultiplied(0, 0, 0, 180);
    let screen =
        to_pos(camera.world_to_screen(annotation.anchor_world, center)) + annotation.offset_px;
    let shadow = screen + egui::vec2(1.0, 1.0);

    painter.text(
        shadow,
        annotation.align,
        &annotation.text,
        font.clone(),
        shadow_color,
    );
    painter.text(
        screen,
        annotation.align,
        &annotation.text,
        font.clone(),
        annotation.color,
    );
}

fn paint_persistent_annotation(
    painter: &egui::Painter,
    camera: &Camera2d,
    center: glam::DVec2,
    annotation: &DimensionAnnotation,
    font: &FontId,
) {
    let Some((start, end)) = annotation.span_world else {
        return;
    };
    let start_screen = to_pos(camera.world_to_screen(start, center));
    let end_screen = to_pos(camera.world_to_screen(end, center));
    let stroke = Stroke::new(1.2, annotation.color);

    painter.line_segment([start_screen, end_screen], stroke);
    painter.circle_filled(start_screen, 2.0, annotation.color);
    painter.circle_filled(end_screen, 2.0, annotation.color);
    paint_annotation(painter, camera, center, annotation, font);
}
