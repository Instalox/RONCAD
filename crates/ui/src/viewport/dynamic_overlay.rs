//! Fusion-style dynamic-input HUD. While a tool is placing geometry and
//! exposes `dynamic_fields`, this overlay floats at the cursor and echoes
//! the digits the user is typing into each field. Tab cycles, Enter commits;
//! the actual keystroke capture lives in the app-side interaction controller.

use egui::{Area, Color32, Frame, Id, Margin, Order, Pos2, Rect, Stroke, Ui, Vec2};
use roncad_tools::{DynamicFieldView, DynamicFieldVisualState};

use super::{project_workplane_point, screen_center};
use crate::shell::ShellContext;
use crate::theme::ThemeColors;

const DYNAMIC_HUD_GAP: f32 = 18.0;
const DYNAMIC_HUD_PAD: f32 = 8.0;
const DYNAMIC_HUD_MIN_WIDTH: f32 = 96.0;
const DYNAMIC_HUD_MAX_WIDTH: f32 = 176.0;
const DYNAMIC_HUD_ROW_HEIGHT: f32 = 18.0;
const DYNAMIC_HUD_HINT_HEIGHT: f32 = 14.0;
const DYNAMIC_HUD_LABEL_CHAR_WIDTH: f32 = 6.6;
const DYNAMIC_HUD_VALUE_CHAR_WIDTH: f32 = 7.4;
const DYNAMIC_HUD_UNIT_WIDTH: f32 = 20.0;
const DYNAMIC_HUD_ROW_GAP: f32 = 18.0;
const DYNAMIC_HUD_FRAME_WIDTH: f32 = 12.0;

pub(super) fn paint(ui: &mut Ui, rect: Rect, shell: &ShellContext<'_>) {
    let views = shell.tool_manager.dynamic_views();
    if views.is_empty() {
        return;
    }

    let Some(cursor_world) = *shell.cursor_world_mm else {
        return;
    };
    let Some(workplane) = shell.project.active_workplane() else {
        return;
    };
    let center = screen_center(rect);
    let Some(cursor_screen) =
        project_workplane_point(shell.camera, center, workplane, cursor_world)
    else {
        return;
    };
    let hud_size = fitted_dynamic_hud_size(dynamic_hud_size(&views), rect);
    let hud_pos = dynamic_hud_pos(cursor_screen, rect, hud_size);

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
                    ui.set_min_width(hud_size.x);
                    ui.set_max_width(hud_size.x);
                    ui.vertical(|ui| {
                        for (index, view) in views.iter().enumerate() {
                            ui.push_id(("dynamic_field", index), |ui| {
                                render_row(ui, view, accent);
                            });
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
            let shown = display_text(view);
            ui.colored_label(
                value_color,
                egui::RichText::new(shown).monospace().strong().size(12.0),
            );
        });
    });
}

fn display_text(view: &DynamicFieldView) -> String {
    if view.active && !view.text.is_empty() {
        format!("[{}]", view.text)
    } else {
        view.text.clone()
    }
}

fn dynamic_hud_pos(cursor: Pos2, rect: Rect, size: Vec2) -> Pos2 {
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

fn dynamic_hud_size(views: &[DynamicFieldView]) -> Vec2 {
    let label_width = views
        .iter()
        .map(|view| view.label.chars().count() as f32 * DYNAMIC_HUD_LABEL_CHAR_WIDTH)
        .fold(0.0, f32::max);
    let value_width = views
        .iter()
        .map(|view| display_text(view).chars().count() as f32 * DYNAMIC_HUD_VALUE_CHAR_WIDTH)
        .fold(0.0, f32::max);
    let row_width = DYNAMIC_HUD_FRAME_WIDTH
        + label_width
        + DYNAMIC_HUD_ROW_GAP
        + value_width
        + DYNAMIC_HUD_UNIT_WIDTH;
    let width = row_width.clamp(DYNAMIC_HUD_MIN_WIDTH, DYNAMIC_HUD_MAX_WIDTH);

    Vec2::new(
        width,
        9.0 + views.len() as f32 * DYNAMIC_HUD_ROW_HEIGHT + DYNAMIC_HUD_HINT_HEIGHT,
    )
}

fn fitted_dynamic_hud_size(size: Vec2, rect: Rect) -> Vec2 {
    Vec2::new(
        size.x.min((rect.width() - DYNAMIC_HUD_PAD * 2.0).max(0.0)),
        size.y.min((rect.height() - DYNAMIC_HUD_PAD * 2.0).max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};
    use roncad_tools::{DynamicFieldView, DynamicFieldVisualState};

    use super::{dynamic_hud_pos, dynamic_hud_size, fitted_dynamic_hud_size};

    fn sample_views() -> Vec<DynamicFieldView> {
        vec![
            DynamicFieldView {
                label: "Length",
                unit: "mm",
                text: "8.602".to_string(),
                active: true,
                state: DynamicFieldVisualState::Valid,
            },
            DynamicFieldView {
                label: "Angle",
                unit: "deg",
                text: "144.5".to_string(),
                active: false,
                state: DynamicFieldVisualState::Preview,
            },
        ]
    }

    #[test]
    fn dynamic_hud_flips_left_when_near_right_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 280.0));
        let cursor = pos2(360.0, 120.0);
        let size = fitted_dynamic_hud_size(dynamic_hud_size(&sample_views()), rect);

        let pos = dynamic_hud_pos(cursor, rect, size);

        assert!(pos.x + size.x <= cursor.x);
    }

    #[test]
    fn dynamic_hud_flips_up_when_near_bottom_edge() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 280.0));
        let cursor = pos2(120.0, 250.0);
        let size = fitted_dynamic_hud_size(dynamic_hud_size(&sample_views()), rect);

        let pos = dynamic_hud_pos(cursor, rect, size);

        assert!(pos.y + size.y <= cursor.y);
    }

    #[test]
    fn dynamic_hud_stays_inside_viewport() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(130.0, 90.0));
        let cursor = pos2(120.0, 80.0);
        let size = fitted_dynamic_hud_size(dynamic_hud_size(&sample_views()), rect);

        let pos = dynamic_hud_pos(cursor, rect, size);

        assert!(pos.x >= rect.min.x);
        assert!(pos.y >= rect.min.y);
        assert!(pos.x + size.x <= rect.max.x + 0.1);
        assert!(pos.y + size.y <= rect.max.y + 0.1);
    }

    #[test]
    fn dynamic_hud_stays_compact_for_two_field_line_input() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 280.0));
        let size = fitted_dynamic_hud_size(dynamic_hud_size(&sample_views()), rect);

        assert!(size.x <= 176.0);
        assert!(size.x >= 120.0);
    }
}
