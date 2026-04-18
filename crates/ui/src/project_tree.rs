//! Project tree panel. Shows workplanes, sketches, and bodies.

use egui::{CollapsingHeader, Panel, Ui};
use egui_phosphor::regular as ph;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::left("project_tree")
        .default_size(200.0)
        .min_size(160.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.heading("Project");
            ui.separator();

            CollapsingHeader::new("Origin")
                .default_open(true)
                .show(ui, |ui| {
                    for (_, plane) in shell.project.workplanes.iter() {
                        ui.label(format!("{} plane", plane.name));
                    }
                });

            CollapsingHeader::new("Sketches")
                .default_open(true)
                .show(ui, |ui| {
                    if shell.project.sketches.is_empty() {
                        ui.colored_label(ThemeColors::TEXT_DIM, "(none yet)");
                    } else {
                        for (id, sketch) in shell.project.sketches.iter() {
                            let marker = if shell.project.active_sketch == Some(id) {
                                ph::DOT
                            } else {
                                ph::DOT_OUTLINE
                            };
                            ui.label(format!(
                                "{marker} {} ({} entities)",
                                sketch.name,
                                sketch.entities.len()
                            ));
                        }
                    }
                });

            CollapsingHeader::new("Bodies")
                .default_open(true)
                .show(ui, |ui| {
                    ui.colored_label(ThemeColors::TEXT_DIM, "(none yet)");
                });
        });
}
