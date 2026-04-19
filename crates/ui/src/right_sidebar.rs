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
        .default_size(272.0)
        .min_size(228.0)
        .frame(
            Frame::new()
                .fill(ThemeColors::BG_PANEL)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR)),
        )
        .show_inside(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);

                    section(ui, "Browser", ph::ROWS, |ui| {
                        project_tree::render_browser_section(ui, shell, response);
                    });
                    section(ui, "Inspector", ph::SIDEBAR_SIMPLE, |ui| {
                        inspector::render_inspector_section(ui, shell, response);
                    });
                    section(ui, "Constraints", ph::LIST_CHECKS, |ui| {
                        placeholder_row(ui, "No constraints yet.");
                    });
                    section(ui, "Export", ph::EXPORT, |ui| {
                        placeholder_row(ui, "STL Export Coming Soon");
                    });
                });
        });
}

fn section(ui: &mut Ui, title: &str, icon: &str, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(ThemeColors::BG_HEADER)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(icon).size(12.5));
                ui.label(
                    RichText::new(title)
                        .color(ThemeColors::TEXT)
                        .size(12.5)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(
                        ThemeColors::TEXT_FAINT,
                        RichText::new(ph::DOTS_THREE).size(12.0),
                    );
                });
            });
        });

    Frame::new()
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, add_contents);

    ui.add_space(2.0);
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT),
    );
    ui.add_space(6.0);
}

fn placeholder_row(ui: &mut Ui, label: &str) {
    ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(label).size(12.0));
}
