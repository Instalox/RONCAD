//! Bottom status bar: coordinates, zoom level, active-tool hint.

use egui::{Panel, Ui};
use roncad_tools::ToolPreview;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::bottom("status_bar")
        .exact_size(24.0)
        .show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(6.0);
                match shell.cursor_world_mm.as_ref() {
                    Some(p) => ui.colored_label(
                        ThemeColors::TEXT,
                        format!("X {:>8.3} mm   Y {:>8.3} mm", p.x, p.y),
                    ),
                    None => ui.colored_label(ThemeColors::TEXT_DIM, "X —   Y —"),
                };

                ui.separator();
                ui.colored_label(
                    ThemeColors::TEXT_DIM,
                    format!("Zoom {:.2} px/mm", shell.camera.pixels_per_mm),
                );

                if let Some(snap) = shell.snap_result.as_ref() {
                    if let Some(kind) = snap.kind {
                        ui.separator();
                        ui.colored_label(ThemeColors::TEXT_DIM, "Snap");
                        ui.colored_label(ThemeColors::ACCENT, kind.label());
                    }
                }

                ui.separator();
                let kind = shell.tool_manager.active_kind();
                ui.colored_label(ThemeColors::TEXT_DIM, kind.hint());

                if let ToolPreview::Measurement { start, end } = shell.tool_manager.preview() {
                    ui.separator();
                    ui.colored_label(
                        ThemeColors::ACCENT,
                        format!("Measure {:.3} mm", start.distance(end)),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);
                    ui.colored_label(ThemeColors::ACCENT, kind.label());
                    ui.colored_label(ThemeColors::TEXT_DIM, "Tool:");
                });
            });
        });
}
