//! Right dock stack. Hosts the browser, inspector, constraints, and export
//! sections in a single dense rail aligned with the concept references.

use egui::{Frame, Margin, Panel, RichText, ScrollArea, Stroke, Ui};
use egui_phosphor::regular as ph;

use crate::inspector;
use crate::project_tree;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    Panel::right("right_sidebar")
        .default_size(264.0)
        .min_size(220.0)
        .show_inside(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    section(ui, "Browser", ph::ROWS, true, |ui| {
                        project_tree::render_browser_section(ui, shell, response);
                    });
                    ui.add_space(6.0);
                    section(ui, "Inspector", ph::SIDEBAR_SIMPLE, true, |ui| {
                        inspector::render_inspector_section(ui, shell, response);
                    });
                    ui.add_space(6.0);
                    section(ui, "Constraints", ph::LIST_CHECKS, false, |ui| {
                        ui.colored_label(ThemeColors::TEXT_DIM, "No constraints yet.");
                    });
                    ui.add_space(6.0);
                    section(ui, "Export", ph::EXPORT, false, |ui| {
                        let _ = ui.add_enabled(
                            false,
                            egui::Button::new("STL Export Coming Soon"),
                        );
                    });
                });
        });
}

fn section(
    ui: &mut Ui,
    title: &str,
    icon: &str,
    active: bool,
    add_contents: impl FnOnce(&mut Ui),
) {
    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
        .inner_margin(Margin::same(0))
        .show(ui, |ui| {
            Frame::new()
                .fill(if active {
                    ThemeColors::BG_HEADER_ACTIVE
                } else {
                    ThemeColors::BG_HEADER
                })
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            if active {
                                ThemeColors::TEXT
                            } else {
                                ThemeColors::TEXT_DIM
                            },
                            RichText::new(icon),
                        );
                        ui.label(
                            RichText::new(title).color(if active {
                                ThemeColors::TEXT
                            } else {
                                ThemeColors::TEXT_DIM
                            }),
                        );
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.colored_label(ThemeColors::TEXT_DIM, ph::DOTS_THREE);
                            },
                        );
                    });
                });

            Frame::new()
                .inner_margin(Margin::symmetric(8, 8))
                .show(ui, add_contents);
        });
}
