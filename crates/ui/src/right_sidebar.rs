//! Right dock stack. Hosts the browser, inspector, constraints, and export
//! sections in a single dense rail aligned with the concept references.

use egui::{Frame, Margin, Panel, RichText, ScrollArea, Stroke, Ui};
use egui_phosphor::regular as ph;

use crate::constraints;
use crate::inspector;
use crate::project_tree;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    let browser_badge =
        (!shell.project.sketches.is_empty()).then(|| shell.project.sketches.len().to_string());
    let constraints_badge = shell
        .project
        .active_sketch()
        .map(|sketch| sketch.constraints.len() + sketch.dimensions.len())
        .filter(|count| *count > 0)
        .map(|count| count.to_string());

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

                    section(ui, "Browser", ph::ROWS, browser_badge.as_deref(), |ui| {
                        project_tree::render_browser_section(ui, shell, response);
                    });
                    section(ui, "Inspector", ph::SIDEBAR_SIMPLE, None, |ui| {
                        inspector::render_inspector_section(ui, shell, response);
                    });
                    section(
                        ui,
                        "Constraints",
                        ph::LIST_CHECKS,
                        constraints_badge.as_deref(),
                        |ui| {
                            constraints::render_constraints_section(ui, shell, response);
                        },
                    );
                    section(ui, "Export", ph::EXPORT, None, |ui| {
                        export_row(ui, "STL");
                        export_row(ui, "STEP");
                        export_row(ui, "DXF");
                    });
                });
        });
}

fn section(
    ui: &mut Ui,
    title: &str,
    icon: &str,
    badge: Option<&str>,
    add_contents: impl FnOnce(&mut Ui),
) {
    Frame::new()
        .fill(ThemeColors::BG_HEADER)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.push_id(("sidebar_section_header", title), |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(icon).size(12.5));
                    ui.label(
                        RichText::new(title)
                            .color(ThemeColors::TEXT)
                            .size(12.5)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    if let Some(badge) = badge {
                        section_badge(ui, badge);
                    }
                    ui.push_id("sidebar_section_menu", |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(
                                ThemeColors::TEXT_FAINT,
                                RichText::new(ph::DOTS_THREE).size(12.0),
                            );
                        });
                    });
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

fn section_badge(ui: &mut Ui, badge: &str) {
    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .inner_margin(Margin::symmetric(6, 2))
        .corner_radius(9.0_f32)
        .show(ui, |ui| {
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                RichText::new(badge).monospace().size(10.5),
            );
        });
}

fn export_row(ui: &mut Ui, label: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(ThemeColors::TEXT_MID, RichText::new(ph::EXPORT).size(12.0));
        ui.label(
            RichText::new(format!("{label} · coming soon"))
                .size(12.0)
                .color(ThemeColors::TEXT_MID),
        );
    });
}
