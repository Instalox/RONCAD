use egui::{Align, Frame, Margin, RichText, Stroke, Ui};
use egui_phosphor::regular as ph;
use roncad_geometry::{closed_profiles, SketchDimension};

use crate::shell::ShellContext;
use crate::theme::ThemeColors;

pub fn render_constraints_section(ui: &mut Ui, shell: &ShellContext<'_>) {
    let Some(sketch) = shell.project.active_sketch() else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("No active sketch.").size(12.0),
        );
        return;
    };

    let profile_count = closed_profiles(sketch).len();
    let dimension_count = sketch.dimensions.len();

    stat_row(ui, "Sketch", &sketch.name, ThemeColors::TEXT);
    stat_row(
        ui,
        "Selection",
        &format!("{} selected", shell.selection.len()),
        ThemeColors::TEXT_MID,
    );
    stat_row(
        ui,
        "Dimensions",
        &dimension_count.to_string(),
        if dimension_count > 0 {
            ThemeColors::ACCENT_AMBER
        } else {
            ThemeColors::TEXT_MID
        },
    );
    stat_row(
        ui,
        "Closed profiles",
        &profile_count.to_string(),
        if profile_count > 0 {
            ThemeColors::ACCENT
        } else {
            ThemeColors::TEXT_MID
        },
    );
    stat_row(ui, "Solver", "Not implemented yet", ThemeColors::TEXT_DIM);

    ui.add_space(6.0);
    if dimension_count == 0 {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("Persistent sketch dimensions will appear here.").size(11.5),
        );
    } else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("Persistent dimensions").size(11.0),
        );
        ui.add_space(3.0);
        for (index, (_, dimension)) in sketch.iter_dimensions().take(3).enumerate() {
            ui.push_id(("constraint_dimension", index), |ui| {
                dimension_row(ui, dimension);
            });
        }
        let remaining = dimension_count.saturating_sub(3);
        if remaining > 0 {
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                RichText::new(format!("+{remaining} more"))
                    .size(10.5)
                    .monospace(),
            );
        }
    }
}

fn stat_row(ui: &mut Ui, label: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [84.0, 18.0],
            egui::Label::new(RichText::new(label).size(11.0).color(ThemeColors::TEXT_DIM)),
        );
        ui.colored_label(color, RichText::new(value).size(11.5).monospace());
    });
}

fn dimension_row(ui: &mut Ui, dimension: &SketchDimension) {
    let (label, value) = match dimension {
        SketchDimension::Distance { start, end } => {
            ("Distance", format!("{:.3} mm", start.distance(*end)))
        }
    };

    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(6, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    ThemeColors::ACCENT_AMBER,
                    RichText::new(ph::RULER).size(11.5),
                );
                ui.label(RichText::new(label).size(11.5).color(ThemeColors::TEXT));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.colored_label(
                        ThemeColors::ACCENT_AMBER,
                        RichText::new(value).size(11.0).monospace(),
                    );
                });
            });
        });
}
