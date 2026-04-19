use egui::{Area, Frame, Id, Margin, Order, Pos2, Rect, RichText, Stroke, Ui};
use roncad_geometry::HoverTarget;
use roncad_tools::ActiveToolKind;

use crate::dimensions;
use crate::shell::ShellContext;
use crate::theme::ThemeColors;

const HUD_PAD_X: f32 = 12.0;
const HUD_PAD_Y: f32 = 10.0;

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &ShellContext<'_>,
    hovered_target: Option<&HoverTarget>,
) {
    let kind = shell.tool_manager.active_kind();
    let accent = ThemeColors::tool_accent(kind);
    let max_width = (rect.width() - HUD_PAD_X * 2.0).max(160.0);
    let hover_text = dimensions::hovered_target_summary(shell.project, hovered_target);
    let step_text = if kind == ActiveToolKind::Extrude && shell.extrude_hud.is_open() {
        "Set a distance, then apply the extrusion.".to_string()
    } else {
        shell.tool_manager.step_hint()
    };

    Area::new(Id::new("viewport_hint_strip"))
        .order(Order::Foreground)
        .fixed_pos(Pos2::new(rect.min.x + HUD_PAD_X, rect.min.y + HUD_PAD_Y))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(max_width);
            Frame::new()
                .fill(ThemeColors::BG_PANEL_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(10, 6))
                .corner_radius(3.0_f32)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                    ui.push_id("viewport_hint_content", |ui| {
                        ui.vertical(|ui| {
                            ui.push_id("viewport_hint_row", |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(
                                        accent,
                                        RichText::new(kind.label()).size(11.5).strong(),
                                    );
                                    hud_sep(ui);
                                    ui.colored_label(
                                        ThemeColors::TEXT_MID,
                                        RichText::new(step_text).size(11.5),
                                    );

                                    if !shell.tool_manager.dynamic_fields().is_empty() {
                                        hud_sep(ui);
                                        hud_segment(ui, "Tab", "fields");
                                        hud_segment(ui, "Enter", "commit");
                                    }

                                    if let Some((key, label)) = modifier_hint(kind) {
                                        hud_sep(ui);
                                        hud_segment(ui, key, label);
                                    }

                                    if kind == ActiveToolKind::Extrude
                                        && shell.extrude_hud.is_open()
                                    {
                                        hud_sep(ui);
                                        hud_segment(ui, "Esc", "cancel");
                                    }

                                    hud_sep(ui);
                                    hud_segment(ui, "middle", "pan");
                                    hud_segment(ui, "scroll", "zoom");
                                });
                            });

                            if let Some(hover_text) = hover_text.as_deref() {
                                ui.add_space(1.0);
                                ui.colored_label(
                                    ThemeColors::ACCENT_AMBER,
                                    RichText::new(hover_text).monospace().size(10.5),
                                );
                            }
                        });
                    });
                });
        });
}

fn modifier_hint(kind: ActiveToolKind) -> Option<(&'static str, &'static str)> {
    match kind {
        ActiveToolKind::Line => Some(("Shift", "ortho")),
        ActiveToolKind::Rectangle => Some(("Shift", "square")),
        ActiveToolKind::Select => Some(("Ctrl", "add")),
        ActiveToolKind::Extrude => Some(("Enter", "apply")),
        _ => None,
    }
}

fn hud_segment(ui: &mut Ui, key: &str, label: &str) {
    keycap(ui, key);
    ui.colored_label(ThemeColors::TEXT_DIM, RichText::new(label).size(11.0));
}

fn hud_sep(ui: &mut Ui) {
    ui.colored_label(
        ThemeColors::TEXT_FAINT,
        RichText::new("|").monospace().size(10.5),
    );
}

fn keycap(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(ThemeColors::BG_DEEP)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(5, 1))
        .corner_radius(3.0_f32)
        .show(ui, |ui| {
            ui.colored_label(
                ThemeColors::TEXT,
                RichText::new(text).monospace().size(10.0),
            );
        });
}
