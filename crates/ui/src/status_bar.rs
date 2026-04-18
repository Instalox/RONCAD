//! Bottom status bar: coordinates, zoom level, active-tool hint.

use egui::{Panel, Ui};
use roncad_tools::ToolPreview;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::bottom("status_bar")
        .exact_size(24.0)
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                let kind = shell.tool_manager.active_kind();
                ui.colored_label(ThemeColors::TEXT_DIM, "Mode");
                ui.colored_label(ThemeColors::tool_accent(kind), kind.label());

                ui.separator();
                match shell.cursor_world_mm.as_ref() {
                    Some(p) => ui.colored_label(
                        ThemeColors::TEXT,
                        format!("X {:>8.3} mm   Y {:>8.3} mm", p.x, p.y),
                    ),
                    None => ui.colored_label(ThemeColors::TEXT_DIM, "X —   Y —"),
                };

                if let Some(snap) = shell.snap_result.as_ref() {
                    if let Some(kind) = snap.kind {
                        ui.separator();
                        ui.colored_label(ThemeColors::TEXT_DIM, "Snap");
                        ui.colored_label(ThemeColors::ACCENT, kind.label());
                    }
                }

                ui.separator();
                ui.colored_label(ThemeColors::TEXT_DIM, shell.tool_manager.step_hint());

                if let Some(summary) = preview_summary(shell.tool_manager.preview()) {
                    ui.separator();
                    ui.colored_label(ThemeColors::tool_accent(kind), summary);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);
                    ui.colored_label(
                        ThemeColors::TEXT_DIM,
                        format!("Zoom {:.2} px/mm", shell.camera.pixels_per_mm),
                    );
                });
            });
        });
}

fn preview_summary(preview: ToolPreview) -> Option<String> {
    match preview {
        ToolPreview::None => None,
        ToolPreview::Line { start, end } => {
            let delta = end - start;
            Some(format!(
                "L {:.3} mm   dX {:.3}   dY {:.3}   A {:.1} deg",
                start.distance(end),
                delta.x.abs(),
                delta.y.abs(),
                delta.y.atan2(delta.x).to_degrees(),
            ))
        }
        ToolPreview::Rectangle { corner_a, corner_b } => {
            let size = (corner_b - corner_a).abs();
            Some(format!(
                "W {:.3} mm   H {:.3} mm   Area {:.3} mm^2",
                size.x,
                size.y,
                size.x * size.y,
            ))
        }
        ToolPreview::Circle { radius, .. } => Some(format!(
            "R {:.3} mm   D {:.3} mm   C {:.3} mm",
            radius,
            radius * 2.0,
            std::f64::consts::TAU * radius,
        )),
        ToolPreview::Measurement { start, end } => {
            Some(format!("Measure {:.3} mm", start.distance(end)))
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_tools::ToolPreview;

    use super::preview_summary;

    #[test]
    fn line_preview_reports_length_delta_and_angle() {
        let summary = preview_summary(ToolPreview::Line {
            start: dvec2(0.0, 0.0),
            end: dvec2(3.0, 4.0),
        });

        assert_eq!(
            summary.as_deref(),
            Some("L 5.000 mm   dX 3.000   dY 4.000   A 53.1 deg")
        );
    }

    #[test]
    fn rectangle_preview_reports_area() {
        let summary = preview_summary(ToolPreview::Rectangle {
            corner_a: dvec2(1.0, 2.0),
            corner_b: dvec2(6.0, 5.0),
        });

        assert_eq!(
            summary.as_deref(),
            Some("W 5.000 mm   H 3.000 mm   Area 15.000 mm^2")
        );
    }

    #[test]
    fn circle_preview_reports_circumference() {
        let summary = preview_summary(ToolPreview::Circle {
            center: dvec2(0.0, 0.0),
            radius: 2.0,
        });

        assert_eq!(
            summary.as_deref(),
            Some("R 2.000 mm   D 4.000 mm   C 12.566 mm")
        );
    }
}
