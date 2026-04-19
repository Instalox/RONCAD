//! Fusion-style dynamic-input HUD. While a tool is placing geometry and
//! exposes `dynamic_fields`, this overlay floats at the cursor and echoes
//! the digits the user is typing into each field. Tab cycles, Enter commits;
//! the actual keystroke capture lives in the app-side interaction controller.

use egui::{Area, Color32, Frame, Id, Margin, Order, Pos2, Rect, Stroke, Ui};
use roncad_tools::DynamicFieldVisualState;

use super::{screen_center, to_pos};
use crate::shell::ShellContext;
use crate::theme::ThemeColors;

pub(super) fn paint(ui: &mut Ui, rect: Rect, shell: &ShellContext<'_>) {
    let views = shell.tool_manager.dynamic_views();
    if views.is_empty() {
        return;
    }

    let Some(cursor_world) = *shell.cursor_world_mm else {
        return;
    };
    let center = screen_center(rect);
    let cursor_screen = to_pos(shell.camera.world_to_screen(cursor_world, center));
    let hud_pos = Pos2::new(cursor_screen.x + 18.0, cursor_screen.y + 18.0);

    let accent = ThemeColors::tool_accent(shell.tool_manager.active_kind());

    Area::new(Id::new("dynamic_input_hud"))
        .order(Order::Foreground)
        .fixed_pos(hud_pos)
        .constrain_to(rect)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(8, 6))
                .corner_radius(3.0_f32)
                .show(ui, |ui| {
                    ui.set_min_width(150.0);
                    ui.vertical(|ui| {
                        for view in &views {
                            render_row(ui, view, accent);
                        }
                        ui.add_space(2.0);
                        ui.colored_label(
                            ThemeColors::TEXT_DIM,
                            "Tab next · Shift+Tab prev · Enter commit · Esc clear/cancel",
                        );
                    });
                });
        });
}

fn render_row(ui: &mut Ui, view: &roncad_tools::DynamicFieldView, accent: Color32) {
    ui.horizontal(|ui| {
        let label_color = if view.active {
            accent
        } else {
            ThemeColors::TEXT_DIM
        };
        ui.colored_label(label_color, view.label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(ThemeColors::TEXT_DIM, view.unit);
            let value_color = match view.state {
                DynamicFieldVisualState::Preview => {
                    if view.active {
                        accent
                    } else {
                        ThemeColors::TEXT
                    }
                }
                DynamicFieldVisualState::Valid => accent,
                DynamicFieldVisualState::Incomplete => ThemeColors::ACCENT_AMBER,
                DynamicFieldVisualState::InvalidParse
                | DynamicFieldVisualState::InvalidGeometry => Color32::from_rgb(0xD9, 0x62, 0x62),
            };
            let shown = if view.active && !view.text.is_empty() {
                format!("[{}]", view.text)
            } else {
                view.text.clone()
            };
            ui.colored_label(value_color, egui::RichText::new(shown).monospace().strong());
        });
    });
}
