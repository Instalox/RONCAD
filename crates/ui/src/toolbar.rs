//! Top toolbar: file/edit/view menus and project title.

use egui::{Panel, Ui};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    Panel::top("toolbar")
        .exact_size(36.0)
        .show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(4.0);
                ui.colored_label(ThemeColors::ACCENT, "RONCAD");
                ui.separator();

                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            ui.close();
                        }
                        if ui.button("Open…").clicked() {
                            ui.close();
                        }
                        if ui.button("Save").clicked() {
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            response.quit_requested = true;
                            ui.close();
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        let _ = ui.button("Undo");
                        let _ = ui.button("Redo");
                    });
                    ui.menu_button("View", |ui| {
                        let _ = ui.button("Fit");
                        let _ = ui.button("Top");
                        let _ = ui.button("Front");
                        let _ = ui.button("Right");
                    });
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(ThemeColors::TEXT_DIM, &shell.project.name);
                });
            });
        });
}
