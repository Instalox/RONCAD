use egui::{
    text::{CCursor, CCursorRange},
    Area, Button, Frame, Id, Key, Margin, Order, Pos2, Rect, RichText, Stroke, TextEdit, Ui,
};
use egui_phosphor::regular as ph;
use roncad_core::{command::AppCommand, units::LengthMm};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const EXTRUDE_PANEL_WIDTH: f32 = 236.0;
const EXTRUDE_PANEL_PAD: f32 = 14.0;

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
) {
    if shell.tool_manager.active_kind() != roncad_tools::ActiveToolKind::Extrude {
        shell.extrude_hud.clear();
        return;
    }

    let Some(draft) = shell.extrude_hud.active().cloned() else {
        return;
    };
    if !shell.project.sketches.contains_key(draft.sketch) {
        shell.extrude_hud.clear();
        return;
    }

    let position = Pos2::new(
        rect.max.x - EXTRUDE_PANEL_WIDTH - EXTRUDE_PANEL_PAD,
        rect.min.y + EXTRUDE_PANEL_PAD,
    );
    let distance_id = Id::new("extrude_distance_input");
    let apply_enabled = shell.extrude_hud.parsed_distance().is_some();
    let enter_pressed = !shell.command_palette.is_open()
        && ui
            .ctx()
            .input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Enter));

    Area::new(Id::new("extrude_overlay"))
        .order(Order::Foreground)
        .fixed_pos(position)
        .constrain_to(rect)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(10, 8))
                .corner_radius(4.0_f32)
                .show(ui, |ui| {
                    ui.set_min_width(EXTRUDE_PANEL_WIDTH);
                    ui.set_max_width(EXTRUDE_PANEL_WIDTH);

                    ui.horizontal(|ui| {
                        ui.colored_label(
                            ThemeColors::ACCENT,
                            RichText::new(ph::ARROW_FAT_LINE_UP).size(12.5),
                        );
                        ui.label(
                            RichText::new("Extrude")
                                .size(12.5)
                                .strong()
                                .color(ThemeColors::TEXT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(
                                ThemeColors::TEXT_DIM,
                                RichText::new("LIVE").monospace().size(9.5),
                            );
                        });
                    });

                    ui.add_space(8.0);
                    value_row(ui, "Profile", "1 face", ThemeColors::TEXT);
                    value_row(
                        ui,
                        "Area",
                        &format!("{:.3} mm^2", draft.profile.area()),
                        ThemeColors::TEXT,
                    );

                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [58.0, 18.0],
                            egui::Label::new(
                                RichText::new("Distance")
                                    .size(11.0)
                                    .color(ThemeColors::TEXT_DIM),
                            ),
                        );
                        let input = ui.add_sized(
                            [122.0, 22.0],
                            TextEdit::singleline(shell.extrude_hud.distance_text_mut())
                                .id(distance_id)
                                .hint_text("10.000")
                                .clip_text(false),
                        );
                        ui.colored_label(ThemeColors::TEXT_DIM, RichText::new("mm").size(10.5));
                        if shell.extrude_hud.take_focus_request() {
                            ui.ctx()
                                .memory_mut(|memory| memory.request_focus(distance_id));
                        }
                        if shell.extrude_hud.take_select_all_request() {
                            select_all_text(
                                ui,
                                distance_id,
                                shell.extrude_hud.distance_text().len(),
                            );
                        }
                        if input.changed() {
                            ui.ctx().request_repaint();
                        }
                    });

                    if !apply_enabled {
                        ui.colored_label(
                            egui::Color32::from_rgb(0xD9, 0x62, 0x62),
                            RichText::new("Distance must be a positive number.").size(10.5),
                        );
                    }

                    ui.add_space(8.0);
                    let mut cancel_clicked = false;
                    let mut apply_clicked = false;
                    ui.horizontal(|ui| {
                        cancel_clicked =
                            ui.add_sized([80.0, 24.0], Button::new("Cancel")).clicked();
                        apply_clicked = ui
                            .add_enabled(
                                apply_enabled,
                                Button::new(
                                    RichText::new("Apply").color(ThemeColors::TEXT).strong(),
                                ),
                            )
                            .clicked();
                    });

                    ui.add_space(4.0);
                    ui.colored_label(
                        ThemeColors::TEXT_DIM,
                        RichText::new("Enter applies · Esc cancels")
                            .size(10.5)
                            .monospace(),
                    );

                    if cancel_clicked {
                        shell.extrude_hud.clear();
                    } else if apply_clicked || enter_pressed {
                        submit(shell, response);
                    }
                });
        });
}

fn value_row(ui: &mut Ui, label: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [58.0, 18.0],
            egui::Label::new(RichText::new(label).size(11.0).color(ThemeColors::TEXT_DIM)),
        );
        ui.colored_label(color, RichText::new(value).size(11.5).monospace());
    });
}

fn submit(shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
    let Some(distance) = shell.extrude_hud.parsed_distance() else {
        return;
    };
    let Some(draft) = shell.extrude_hud.active().cloned() else {
        return;
    };

    response.commands.push(AppCommand::ExtrudeProfile {
        sketch: draft.sketch,
        distance: LengthMm::new(distance),
    });
    shell.extrude_hud.clear();
}

fn select_all_text(ui: &Ui, widget_id: Id, len: usize) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), widget_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
    egui::TextEdit::store_state(ui.ctx(), widget_id, state);
}
