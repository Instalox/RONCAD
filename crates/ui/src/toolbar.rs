//! Top command bar. Keeps the chrome compact and concept-aligned: brand,
//! active sketch selector, compact actions, and mode status.

use egui::{Button, ComboBox, FontId, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2};
use egui_phosphor::regular as ph;
use roncad_core::command::AppCommand;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const TOOLBAR_HEIGHT: f32 = 40.0;
const TOOLBAR_ICON_SIZE: f32 = 24.0;

pub fn render(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    egui::Panel::top("toolbar")
        .exact_size(TOOLBAR_HEIGHT)
        .frame(
            Frame::new()
                .fill(ThemeColors::BG_PANEL)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(10, 6)),
        )
        .show_inside(ui, |ui| {
            let active_tool = shell.tool_manager.active_kind();
            let tool_accent = ThemeColors::tool_accent(active_tool);

            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);

            ui.push_id("toolbar_row", |ui| {
                ui.horizontal(|ui| {
                    brand(ui, tool_accent);
                    toolbar_divider(ui);

                    sketch_selector(ui, shell, response);
                    if icon_button(ui, ph::PLUS, "Create sketch").clicked() {
                        response.commands.push(AppCommand::CreateSketch {
                            name: format!("Sketch {}", shell.project.sketches.len() + 1),
                        });
                    }

                    toolbar_divider(ui);
                    let _ = icon_button(ui, ph::FLOPPY_DISK, "Save project");
                    let _ = icon_button(ui, ph::ARROWS_COUNTER_CLOCKWISE, "Undo");
                    let _ = icon_button(ui, ph::ARROW_CLOCKWISE, "Redo");
                    let _ = icon_button(ui, ph::PROJECTOR_SCREEN, "Fit view");

                    ui.push_id("toolbar_trailing", |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let _ = icon_button(ui, ph::GEAR, "Application settings");
                            mode_chip(
                                ui,
                                active_tool.label(),
                                tool_accent,
                                ThemeColors::tool_accent_dim(active_tool),
                            );
                            ui.label(
                                RichText::new(&shell.project.name)
                                    .size(12.0)
                                    .color(ThemeColors::TEXT_DIM),
                            );
                        });
                    });
                });
            });
        });
}

fn brand(ui: &mut Ui, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.colored_label(
            color,
            RichText::new(ph::CUBE_FOCUS).font(FontId::proportional(15.0)),
        );
        ui.label(
            RichText::new("RONCAD")
                .size(13.0)
                .strong()
                .color(ThemeColors::TEXT),
        );
    });
}

fn sketch_selector(ui: &mut Ui, shell: &ShellContext<'_>, response: &mut ShellResponse) {
    let selected = shell
        .project
        .active_sketch()
        .map_or_else(|| "No Sketch".to_string(), |sketch| sketch.name.clone());

    ComboBox::from_id_salt("active_sketch_selector")
        .width(140.0)
        .selected_text(RichText::new(selected).size(12.0))
        .show_ui(ui, |ui| {
            for (id, sketch) in shell.project.sketches.iter() {
                let selected = shell.project.active_sketch == Some(id);
                if ui.selectable_label(selected, &sketch.name).clicked() && !selected {
                    response.commands.push(AppCommand::SetActiveSketch(id));
                    ui.close();
                }
            }
        });
}

fn icon_button(ui: &mut Ui, icon: &str, hover_text: &str) -> egui::Response {
    ui.add_sized(
        Vec2::new(TOOLBAR_ICON_SIZE, TOOLBAR_ICON_SIZE),
        Button::new(RichText::new(icon).font(FontId::proportional(13.0))),
    )
    .on_hover_text(hover_text)
}

fn toolbar_divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 18.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT),
    );
}

fn mode_chip(ui: &mut Ui, label: &str, color: egui::Color32, dim: egui::Color32) {
    Frame::new()
        .fill(ThemeColors::BG_HEADER_ACTIVE)
        .stroke(Stroke::new(1.0, dim))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.0, color);
                ui.colored_label(color, RichText::new(label).size(11.5));
            });
        });
}
