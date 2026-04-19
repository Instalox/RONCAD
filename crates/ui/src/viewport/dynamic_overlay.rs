//! Fusion-style dynamic-input HUD. While a tool is placing geometry and
//! exposes `dynamic_fields`, this overlay floats at the cursor and echoes
//! the digits the user is typing into each field. Tab cycles, Enter commits;
//! the actual keystroke capture lives in the app-side interaction controller.

use egui::{Area, Color32, Frame, Id, Margin, Order, Pos2, Rect, Stroke, Ui, Vec2};
use roncad_tools::DynamicFieldVisualState;

use super::{screen_center, to_pos};
use crate::shell::ShellContext;
use crate::theme::ThemeColors;

const DYNAMIC_HUD_GAP: f32 = 18.0;
const DYNAMIC_HUD_PAD: f32 = 8.0;
const DYNAMIC_HUD_MIN_WIDTH: f32 = 112.0;
const DYNAMIC_HUD_ROW_HEIGHT: f32 = 20.0;
const DYNAMIC_HUD_HINT_HEIGHT: f32 = 16.0;

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
    let hud_pos = dynamic_hud_pos(cursor_screen, rect, views.len());

    let accent = ThemeColors::tool_accent(shell.tool_manager.active_kind());

    Area::new(Id::new("dynamic_input_hud"))
        .order(Order::Foreground)
        .fixed_pos(hud_pos)
        .constrain_to(rect)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL_ALT_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(6, 4))
                .corner_radius(3.0_f32)
                .show(ui, |ui| {
                    ui.set_min_width(DYNAMIC_HUD_MIN_WIDTH);
                    ui.vertical(|ui| {
                        for view in &views {
                            render_row(ui, view, accent);
                        }
                        ui.add_space(1.0);
                        ui.colored_label(
                            ThemeColors::TEXT_DIM,
                            egui::RichText::new("Tab/Enter/Esc").size(10.5),
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
        ui.colored_label(label_color, egui::RichText::new(view.label).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(
                ThemeColors::TEXT_DIM,
                egui::RichText::new(view.unit).size(11.5),
            );
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
            ui.colored_label(
                value_color,
                egui::RichText::new(shown).monospace().strong().size(12.0),
            );
        });
    });
}

fn dynamic_hud_pos(cursor: Pos2, rect: Rect, field_count: usize) -> Pos2 {
    let size = dynamic_hud_size(field_count);
    let min_x = rect.min.x + DYNAMIC_HUD_PAD;
    let max_x = rect.max.x - size.x - DYNAMIC_HUD_PAD;
    let min_y = rect.min.y + DYNAMIC_HUD_PAD;
    let max_y = rect.max.y - size.y - DYNAMIC_HUD_PAD;

    let prefer_left = cursor.x + DYNAMIC_HUD_GAP + size.x > rect.max.x - DYNAMIC_HUD_PAD;
    let prefer_up = cursor.y + DYNAMIC_HUD_GAP + size.y > rect.max.y - DYNAMIC_HUD_PAD;

    let left_x = cursor.x - DYNAMIC_HUD_GAP - size.x;
    let right_x = cursor.x + DYNAMIC_HUD_GAP;
    let up_y = cursor.y - DYNAMIC_HUD_GAP - size.y;
    let down_y = cursor.y + DYNAMIC_HUD_GAP;

    let x = if prefer_left {
        left_x.clamp(min_x, max_x.max(min_x))
    } else {
        right_x.clamp(min_x, max_x.max(min_x))
    };
    let y = if prefer_up {
        up_y.clamp(min_y, max_y.max(min_y))
    } else {
        down_y.clamp(min_y, max_y.max(min_y))
    };

    Pos2::new(x, y)
}

fn dynamic_hud_size(field_count: usize) -> Vec2 {
    Vec2::new(
        DYNAMIC_HUD_MIN_WIDTH,
        10.0 + field_count as f32 * DYNAMIC_HUD_ROW_HEIGHT + DYNAMIC_HUD_HINT_HEIGHT,
    )
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};

    use super::{dynamic_hud_pos, dynamic_hud_size};

    #[test]
    fn dynamic_hud_flips_left_when_near_right_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 280.0));
        let cursor = pos2(360.0, 120.0);

        let pos = dynamic_hud_pos(cursor, rect, 2);
        let size = dynamic_hud_size(2);

        assert!(pos.x + size.x <= cursor.x);
    }

    #[test]
    fn dynamic_hud_flips_up_when_near_bottom_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 280.0));
        let cursor = pos2(120.0, 250.0);

        let pos = dynamic_hud_pos(cursor, rect, 2);
        let size = dynamic_hud_size(2);

        assert!(pos.y + size.y <= cursor.y);
    }

    #[test]
    fn dynamic_hud_stays_inside_viewport() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(130.0, 90.0));
        let cursor = pos2(120.0, 80.0);

        let pos = dynamic_hud_pos(cursor, rect, 2);
        let size = dynamic_hud_size(2);

        assert!(pos.x >= rect.min.x);
        assert!(pos.y >= rect.min.y);
        assert!(pos.x + size.x <= rect.max.x + 0.1);
        assert!(pos.y + size.y <= rect.max.y + 0.1);
    }
}
