//! Browser section. Shows workplanes, sketches, and bodies in the right rail.

use egui::{CollapsingHeader, RichText, Ui};
use egui_phosphor::regular as ph;
use roncad_core::command::AppCommand;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render_browser_section(
    ui: &mut Ui,
    shell: &ShellContext<'_>,
    response: &mut ShellResponse,
) {
    CollapsingHeader::new("Origin")
        .default_open(true)
        .show(ui, |ui| {
            for (_, plane) in shell.project.workplanes.iter() {
                ui.colored_label(
                    ThemeColors::TEXT_DIM,
                    format!("{} {} plane", ph::SQUARE, plane.name),
                );
            }
        });

    CollapsingHeader::new("Sketches")
        .default_open(true)
        .show(ui, |ui| {
            if shell.project.sketches.is_empty() {
                ui.colored_label(ThemeColors::TEXT_DIM, "(none yet)");
            } else {
                for (id, sketch) in shell.project.sketches.iter() {
                    let active = shell.project.active_sketch == Some(id);
                    let icon = if active { ph::DISC } else { ph::CIRCLE };
                    let text = RichText::new(format!(
                        "{icon} {}   {}",
                        sketch.name,
                        entity_summary(sketch.entities.len())
                    ))
                    .color(if active {
                        ThemeColors::TEXT
                    } else {
                        ThemeColors::TEXT_DIM
                    });
                    let button = egui::Button::selectable(active, text);
                    if ui.add_sized([ui.available_width(), 22.0], button).clicked() {
                        response.commands.push(AppCommand::SetActiveSketch(id));
                    }
                }
            }
        });

    CollapsingHeader::new("Bodies")
        .default_open(true)
        .show(ui, |ui| {
            ui.colored_label(ThemeColors::TEXT_DIM, "(none yet)");
        });
}

fn entity_summary(count: usize) -> String {
    match count {
        1 => "1 entity".to_string(),
        _ => format!("{count} entities"),
    }
}
