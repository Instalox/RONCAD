//! Browser section. Shows workplanes, sketches, and bodies in the right rail.

use egui::{Align, Color32, Layout, RichText, Sense, Ui, UiBuilder, Vec2};
use egui_phosphor::regular as ph;
use roncad_core::command::AppCommand;
use roncad_core::selection::SelectionItem;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const TREE_ROW_HEIGHT: f32 = 24.0;
const TREE_GROUP_HEIGHT: f32 = 22.0;
const TREE_INDENT: f32 = 16.0;

pub fn render_browser_section(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);

    tree_group(ui, "Origin");
    for (_, plane) in shell.project.workplanes.iter() {
        ui.push_id(("workplane_row", plane.name.as_str()), |ui| {
            let _ = tree_row(
                ui,
                TreeRow {
                    depth: 1,
                    glyph: ph::SQUARE,
                    label: format!("{} plane", plane.name),
                    count: None,
                    selected: false,
                    muted: false,
                },
            );
        });
    }

    ui.add_space(2.0);
    tree_group(ui, "Sketches");
    if shell.project.sketches.is_empty() {
        ui.push_id("empty_sketches", |ui| {
            let _ = tree_row(
                ui,
                TreeRow {
                    depth: 1,
                    glyph: ph::SQUARE,
                    label: "(none yet)".to_string(),
                    count: None,
                    selected: false,
                    muted: true,
                },
            );
        });
    } else {
        for (id, sketch) in shell.project.sketches.iter() {
            ui.push_id(("sketch_row", id), |ui| {
                let active = shell.project.active_sketch == Some(id);
                let row = tree_row(
                    ui,
                    TreeRow {
                        depth: 1,
                        glyph: ph::DISC,
                        label: sketch.name.clone(),
                        count: Some(entity_summary(sketch.entities.len())),
                        selected: active,
                        muted: false,
                    },
                );
                if row.clicked() && !active {
                    response.commands.push(AppCommand::SetActiveSketch(id));
                }
            });
        }
    }

    ui.add_space(2.0);
    tree_group(ui, "Bodies");
    if shell.project.bodies.is_empty() {
        ui.push_id("empty_bodies", |ui| {
            let _ = tree_row(
                ui,
                TreeRow {
                    depth: 1,
                    glyph: ph::CUBE,
                    label: "(none yet)".to_string(),
                    count: None,
                    selected: false,
                    muted: true,
                },
            );
        });
    } else {
        for (body_id, body) in shell.project.bodies.iter() {
            ui.push_id(("body_row", body_id), |ui| {
                let selected = shell.selection.contains(&SelectionItem::Body(body_id));
                let row = tree_row(
                    ui,
                    TreeRow {
                        depth: 1,
                        glyph: ph::CUBE,
                        label: body.name.clone(),
                        count: Some(feature_summary(body.feature_count())),
                        selected,
                        muted: false,
                    },
                );
                if row.clicked() && !selected {
                    response.commands.push(AppCommand::SelectBody(body_id));
                }
            });

            for (feature_id, feature) in shell.project.body_features(body_id) {
                ui.push_id(("feature_row", feature_id), |ui| {
                    let count = format!("{:.3} mm", feature.distance_mm().abs());
                    let _ = tree_row(
                        ui,
                        TreeRow {
                            depth: 2,
                            glyph: ph::ARROW_FAT_LINE_UP,
                            label: feature.name().to_string(),
                            count: Some(count),
                            selected: false,
                            muted: false,
                        },
                    );
                });
            }
        }
    }
}

struct TreeRow {
    depth: usize,
    glyph: &'static str,
    label: String,
    count: Option<String>,
    selected: bool,
    muted: bool,
}

fn tree_group(ui: &mut Ui, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), TREE_GROUP_HEIGHT),
        Sense::hover(),
    );
    let mut row_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(("tree_group", title))
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    row_ui.add_space(2.0);
    row_ui.colored_label(
        ThemeColors::TEXT_DIM,
        RichText::new(ph::CARET_DOWN).size(10.0),
    );
    row_ui.add_space(2.0);
    row_ui.colored_label(
        ThemeColors::TEXT_MID,
        RichText::new(title).size(12.0).strong(),
    );
}

fn tree_row(ui: &mut Ui, row: TreeRow) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), TREE_ROW_HEIGHT),
        Sense::click(),
    );

    let fill = if row.selected {
        ThemeColors::ACCENT_DIM.gamma_multiply(0.58)
    } else if response.hovered() {
        ThemeColors::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    if fill != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect.shrink2(egui::vec2(0.0, 1.0)), 3.0, fill);
    }

    let text_color = if row.selected {
        ThemeColors::TEXT
    } else if row.muted {
        ThemeColors::TEXT_DIM
    } else {
        ThemeColors::TEXT_MID
    };
    let glyph_color = if row.selected {
        ThemeColors::TEXT
    } else if row.muted {
        ThemeColors::TEXT_FAINT
    } else {
        ThemeColors::TEXT_DIM
    };
    let count_color = if row.selected {
        ThemeColors::TEXT_MID
    } else {
        ThemeColors::TEXT_DIM
    };

    let inner = rect.shrink2(egui::vec2(6.0, 3.0));
    let mut row_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(("tree_row", row.depth, row.glyph, row.label.as_str()))
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    row_ui.add_space((row.depth as f32) * TREE_INDENT);
    row_ui.colored_label(glyph_color, RichText::new(row.glyph).size(12.0));
    row_ui.add_space(6.0);
    row_ui.label(RichText::new(row.label).size(12.0).color(text_color));
    if let Some(count) = row.count {
        row_ui.push_id("tree_row_count", |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.colored_label(count_color, RichText::new(count).monospace().size(10.5));
            });
        });
    }

    response
}

fn entity_summary(count: usize) -> String {
    format!("{count} ent")
}

fn feature_summary(count: usize) -> String {
    format!("{count} feat")
}
