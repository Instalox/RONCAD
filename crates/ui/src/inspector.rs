//! Inspector section. Shows current selection details and active tool hints.

use egui::{Align, Frame, Margin, RichText, Stroke, Ui};

use crate::dimensions::{self, DimensionValue, EntityDimensions};
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render_inspector_section(
    ui: &mut Ui,
    shell: &ShellContext<'_>,
    _response: &mut ShellResponse,
) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);

    if shell.selection.is_empty() {
        ui.colored_label(
            ThemeColors::TEXT_MID,
            RichText::new("Nothing selected.").size(13.0),
        );
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("Pick an entity in the viewport.").size(12.0),
        );
    } else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new(format!("{} selected", shell.selection.len())).size(11.5),
        );

        let selection_dimensions =
            dimensions::selected_entity_dimensions(shell.project, shell.selection);
        if selection_dimensions.is_empty() {
            ui.add_space(2.0);
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                RichText::new("Selection details are not available for this item yet.").size(12.0),
            );
        } else {
            for entity in selection_dimensions {
                entity_card(ui, &entity);
            }
        }
    }

    ui.add_space(2.0);
    ui.separator();
    ui.add_space(2.0);
    active_tool_block(ui, shell);
}

fn entity_card(ui: &mut Ui, entity: &EntityDimensions) {
    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(entity.kind)
                        .color(ThemeColors::TEXT)
                        .size(12.5)
                        .strong(),
                );
            });
            ui.add_space(4.0);
            for value in &entity.summary {
                property_row(ui, value.label, &format_dimension_value(value));
            }
        });
}

fn active_tool_block(ui: &mut Ui, shell: &ShellContext<'_>) {
    let active = shell.tool_manager.active_kind();
    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                RichText::new("Active tool").size(11.0),
            );
            ui.add_space(2.0);
            ui.colored_label(
                ThemeColors::tool_accent(active),
                RichText::new(active.label()).size(13.0).strong(),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(active.hint())
                    .color(ThemeColors::TEXT_DIM)
                    .size(11.5),
            );
        });
}

fn property_row(ui: &mut Ui, label: &str, formatted: &str) {
    let (value, unit) = split_value_unit(formatted);

    ui.horizontal(|ui| {
        ui.add_sized(
            [58.0, 18.0],
            egui::Label::new(RichText::new(label).color(ThemeColors::TEXT_DIM).size(11.5)),
        );
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_DEEP)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
                .inner_margin(Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.set_min_width(132.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if let Some(unit) = unit {
                            ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(unit).size(10.5));
                            ui.add_space(8.0);
                        }
                        ui.label(
                            RichText::new(value)
                                .monospace()
                                .color(ThemeColors::TEXT)
                                .size(12.0),
                        );
                    });
                });
        });
    });
}

fn format_dimension_value(value: &DimensionValue) -> String {
    if value.label == "Sweep" {
        format!("{:.1} deg", value.value_mm)
    } else {
        value.formatted_value()
    }
}

fn split_value_unit(formatted: &str) -> (&str, Option<&str>) {
    if let Some(value) = formatted.strip_suffix(" mm") {
        (value, Some("mm"))
    } else if let Some(value) = formatted.strip_suffix(" deg") {
        (value, Some("deg"))
    } else {
        (formatted, None)
    }
}
