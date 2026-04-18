//! Inspector section. Shows current selection details and active tool hints.

use egui::Ui;

use crate::dimensions;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render_inspector_section(
    ui: &mut Ui,
    shell: &ShellContext<'_>,
    _response: &mut ShellResponse,
) {
    if shell.selection.is_empty() {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            "Nothing selected.\nPick an entity in the viewport.",
        );
    } else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            format!("{} item(s) selected", shell.selection.len()),
        );
        ui.add_space(6.0);

        let dimensions = dimensions::selected_entity_dimensions(shell.project, shell.selection);
        if dimensions.is_empty() {
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                "Selection details are not available for this item yet.",
            );
        } else {
            for entity in dimensions {
                ui.group(|ui| {
                    ui.label(entity.kind);
                    for value in entity.summary {
                        ui.horizontal(|ui| {
                            ui.colored_label(ThemeColors::TEXT_DIM, value.label);
                            ui.label(value.formatted_value());
                        });
                    }
                });
            }
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.colored_label(ThemeColors::TEXT_DIM, "Active tool");
    let active = shell.tool_manager.active_kind();
    ui.colored_label(ThemeColors::tool_accent(active), active.label());
    ui.colored_label(ThemeColors::TEXT_DIM, active.hint());
}
