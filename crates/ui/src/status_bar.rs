//! Bottom status bar: coordinates, zoom level, active-tool hint.

use egui::{vec2, Align, Frame, Label, Layout, Margin, Rect, RichText, Stroke, Ui, UiBuilder};
use roncad_geometry::{SolveReport, SolveStatus};
use roncad_tools::{ActiveToolKind, ToolPreview};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const STATUS_METRICS_WIDTH: f32 = 184.0;

pub fn render_in_rect(
    ui: &mut Ui,
    rect: Rect,
    shell: &ShellContext<'_>,
    _response: &mut ShellResponse,
) {
    let mut status_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("status_bar")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    status_ui.expand_to_include_rect(rect);
    status_ui.set_clip_rect(rect);

    Frame::new()
        .fill(ThemeColors::BG_PANEL)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
        .inner_margin(Margin::symmetric(10, 4))
        .show(&mut status_ui, |ui| {
            let kind = shell.tool_manager.active_kind();
            ui.set_min_height(rect.height());
            ui.set_min_width(rect.width());
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 0.0);

            ui.push_id("status_bar_row", |ui| {
                ui.horizontal(|ui| {
                    ui.push_id("status_bar_primary", |ui| {
                        ui.horizontal(|ui| {
                            status_pair(
                                ui,
                                "Mode",
                                kind.label(),
                                ThemeColors::tool_accent(kind),
                                false,
                            );
                            status_sep(ui);

                            match shell.cursor_world_mm.as_ref() {
                                Some(p) => {
                                    status_metric(ui, "X", &format!("{:.3}", p.x), "mm");
                                    status_metric(ui, "Y", &format!("{:.3}", p.y), "mm");
                                }
                                None => {
                                    status_pair(ui, "X", "-", ThemeColors::TEXT_DIM, true);
                                    status_pair(ui, "Y", "-", ThemeColors::TEXT_DIM, true);
                                }
                            }

                            if let Some(snap) = shell.snap_result.as_ref() {
                                if let Some(kind) = snap.kind {
                                    status_sep(ui);
                                    status_pair(
                                        ui,
                                        "Snap",
                                        kind.label(),
                                        ThemeColors::ACCENT,
                                        false,
                                    );
                                }
                            }
                        });
                    });

                    let info_width =
                        (ui.available_width() - STATUS_METRICS_WIDTH - ui.spacing().item_spacing.x)
                            .max(0.0);
                    if let Some((info, color)) = status_context(shell) {
                        status_sep(ui);
                        ui.add_sized(
                            vec2(info_width, 16.0),
                            Label::new(RichText::new(info).size(11.5).color(color)).truncate(),
                        );
                    } else if info_width > 0.0 {
                        ui.add_space(info_width);
                    }

                    ui.push_id("status_bar_metrics", |ui| {
                        ui.allocate_ui_with_layout(
                            vec2(STATUS_METRICS_WIDTH, ui.available_height()),
                            Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                status_pair(ui, "Units", "mm", ThemeColors::TEXT, false);
                                status_sep(ui);
                                status_metric(
                                    ui,
                                    "Zoom",
                                    &format!("{:.2}", shell.camera.pixels_per_mm),
                                    "px/mm",
                                );
                            },
                        );
                    });
                });
            });
        });
}

fn status_context(shell: &ShellContext<'_>) -> Option<(String, egui::Color32)> {
    let kind = shell.tool_manager.active_kind();
    if let Some(summary) = preview_summary(shell.tool_manager.preview()) {
        return Some((summary, ThemeColors::tool_accent(kind)));
    }
    if let Some(text) = shell.status_text {
        return Some((
            text.to_string(),
            if shell.status_is_error {
                ThemeColors::ACCENT_AMBER
            } else {
                ThemeColors::ACCENT
            },
        ));
    }
    if kind == ActiveToolKind::Extrude && shell.extrude_hud.is_open() {
        if let Some(draft) = shell.extrude_hud.active() {
            let distance = shell
                .extrude_hud
                .parsed_distance()
                .map(|value| format!("{value:.3} mm"))
                .unwrap_or_else(|| "invalid distance".to_string());
            return Some((
                format!(
                    "Profile {:.3} mm^2   Distance {}",
                    draft.profile.area(),
                    distance
                ),
                ThemeColors::ACCENT,
            ));
        }
    }
    if let Some(report) = shell.last_solve_report {
        if report.status != SolveStatus::Trivial {
            return Some(solve_summary(report));
        }
    }
    if !shell.selection.is_empty() {
        return Some((
            format!("{} selected", shell.selection.len()),
            ThemeColors::TEXT_MID,
        ));
    }

    let hint = status_hint(kind, !shell.tool_manager.dynamic_fields().is_empty());
    (!hint.is_empty()).then_some((hint.to_string(), ThemeColors::TEXT_DIM))
}

fn solve_summary(report: &SolveReport) -> (String, egui::Color32) {
    let label = match report.status {
        SolveStatus::Converged => "Solve converged",
        SolveStatus::MaxItersReached => "Solve max iters",
        SolveStatus::Trivial => "Solve trivial",
    };
    let color = match report.status {
        SolveStatus::Converged => ThemeColors::ACCENT,
        SolveStatus::MaxItersReached => ThemeColors::ACCENT_AMBER,
        SolveStatus::Trivial => ThemeColors::TEXT_DIM,
    };

    (
        format!(
            "{label}   {} iters   r={:.2e}",
            report.iterations, report.final_residual_norm
        ),
        color,
    )
}

fn status_hint(kind: ActiveToolKind, dynamic_active: bool) -> &'static str {
    match kind {
        ActiveToolKind::Select => "Click or box-select",
        ActiveToolKind::Pan => "Middle orbit, right pan",
        ActiveToolKind::Line if dynamic_active => "Tab or Enter edits",
        ActiveToolKind::Line => "Click first point",
        ActiveToolKind::Rectangle if dynamic_active => "Type width and height",
        ActiveToolKind::Rectangle => "Click first corner",
        ActiveToolKind::Circle if dynamic_active => "Type radius",
        ActiveToolKind::Circle => "Click center",
        ActiveToolKind::Arc => "Center, start, end",
        ActiveToolKind::Fillet => "Pick corner, then radius",
        ActiveToolKind::Dimension => "Pick points to dimension",
        ActiveToolKind::Extrude => "Click a closed profile",
        ActiveToolKind::Revolve => "Click a closed profile",
    }
}

fn status_metric(ui: &mut Ui, label: &str, value: &str, unit: &str) {
    status_pair(ui, label, value, ThemeColors::TEXT, true);
    ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(unit).size(10.5));
}

fn status_pair(ui: &mut Ui, label: &str, value: &str, color: egui::Color32, monospace: bool) {
    ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(label).size(10.5));
    let text = RichText::new(value).size(11.5).color(color);
    if monospace {
        ui.label(text.monospace());
    } else {
        ui.label(text);
    }
}

fn status_sep(ui: &mut Ui) {
    ui.colored_label(
        ThemeColors::TEXT_FAINT,
        RichText::new("|").monospace().size(10.5),
    );
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
        ToolPreview::ArcRadius { radius, .. } => Some(format!("Arc radius {:.3} mm", radius)),
        ToolPreview::Arc {
            radius,
            sweep_angle,
            ..
        } => Some(format!(
            "Arc R {:.3} mm   Sweep {:.1} deg   Length {:.3} mm",
            radius,
            sweep_angle.abs().to_degrees(),
            radius * sweep_angle.abs(),
        )),
        ToolPreview::FilletHover {
            radius, max_radius, ..
        } => Some(format!(
            "Fillet candidate   Preview R {:.3} mm   Max R {:.3} mm",
            radius, max_radius,
        )),
        ToolPreview::Fillet {
            radius,
            sweep_angle,
            ..
        } => Some(format!(
            "Fillet R {:.3} mm   Sweep {:.1} deg",
            radius,
            sweep_angle.abs().to_degrees(),
        )),
        ToolPreview::Measurement { start, end } => {
            Some(format!("Measure {:.3} mm", start.distance(end)))
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_geometry::{SolveReport, SolveStatus};
    use roncad_tools::ToolPreview;

    use super::{preview_summary, solve_summary};

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

    #[test]
    fn arc_radius_preview_reports_radius() {
        let summary = preview_summary(ToolPreview::ArcRadius {
            center: dvec2(0.0, 0.0),
            radius: 8.0,
            rim: dvec2(8.0, 0.0),
        });

        assert_eq!(summary.as_deref(), Some("Arc radius 8.000 mm"));
    }

    #[test]
    fn arc_preview_reports_radius_sweep_and_length() {
        let summary = preview_summary(ToolPreview::Arc {
            center: dvec2(0.0, 0.0),
            radius: 10.0,
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        });

        assert_eq!(
            summary.as_deref(),
            Some("Arc R 10.000 mm   Sweep 90.0 deg   Length 15.708 mm")
        );
    }

    #[test]
    fn fillet_preview_reports_radius_and_sweep() {
        let summary = preview_summary(ToolPreview::Fillet {
            trim_a: (dvec2(10.0, 0.0), dvec2(2.0, 0.0)),
            trim_b: (dvec2(0.0, 10.0), dvec2(0.0, 2.0)),
            center: dvec2(2.0, 2.0),
            radius: 2.0,
            start_angle: -std::f64::consts::FRAC_PI_2,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        });

        assert_eq!(
            summary.as_deref(),
            Some("Fillet R 2.000 mm   Sweep 90.0 deg")
        );
    }

    #[test]
    fn fillet_hover_reports_preview_and_max_radius() {
        let summary = preview_summary(ToolPreview::FilletHover {
            corner: dvec2(0.0, 0.0),
            trim_a: (dvec2(10.0, 0.0), dvec2(2.0, 0.0)),
            trim_b: (dvec2(0.0, 10.0), dvec2(0.0, 2.0)),
            center: dvec2(2.0, 2.0),
            radius: 2.0,
            start_angle: -std::f64::consts::FRAC_PI_2,
            sweep_angle: std::f64::consts::FRAC_PI_2,
            max_radius: 10.0,
        });

        assert_eq!(
            summary.as_deref(),
            Some("Fillet candidate   Preview R 2.000 mm   Max R 10.000 mm")
        );
    }

    #[test]
    fn solve_summary_formats_nontrivial_reports() {
        let (text, _) = solve_summary(&SolveReport {
            status: SolveStatus::Converged,
            iterations: 7,
            final_residual_norm: 1.25e-9,
        });

        assert_eq!(text, "Solve converged   7 iters   r=1.25e-9");
    }
}
