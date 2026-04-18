//! Paints read-only dimension annotations for the current selection.
//! This is a Milestone 2 bridge before editable constraint dimensions land.

use egui::{Color32, FontId, Rect};
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;

use super::{screen_center, to_pos};
use crate::dimensions::{self, DimensionAnnotation};
use crate::theme::ThemeColors;

pub(super) fn paint(
    painter: &egui::Painter,
    rect: Rect,
    camera: &Camera2d,
    project: &Project,
    selection: &Selection,
) {
    let center = screen_center(rect);
    let font = FontId::monospace(11.0);

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
    let screen = to_pos(camera.world_to_screen(annotation.anchor_world, center))
        + annotation.offset_px;
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
        ThemeColors::TEXT,
    );
}
