//! Right-hand inspector: properties of the current selection.

use egui::{Panel, Ui};

use crate::dimensions;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::right("inspector")
        .default_size(260.0)
        .min_size(200.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.heading("Inspector");
            ui.separator();

            if shell.selection.is_empty() {
                ui.colored_label(
                    ThemeColors::TEXT_DIM,
                    "Nothing selected.\nPick an entity in the viewport.",
                );
            } else {
                ui.label(format!("{} item(s) selected", shell.selection.len()));
                let dimensions =
                    dimensions::selected_entity_dimensions(shell.project, shell.selection);

                if dimensions.is_empty() {
                    ui.add_space(8.0);
                    ui.colored_label(
                        ThemeColors::TEXT_DIM,
                        "Selection details are not available for this item yet.",
                    );
                } else {
                    ui.add_space(8.0);
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

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Active tool");
            let active = shell.tool_manager.active_kind();
            ui.label(active.label());
            ui.colored_label(ThemeColors::TEXT_DIM, active.hint());
        });
}
