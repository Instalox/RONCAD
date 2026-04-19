//! Top command bar. Keeps the chrome compact and concept-aligned: brand,
//! active sketch selector, compact actions, and mode status.

use egui::{
    os::OperatingSystem, Button, ComboBox, FontId, Frame, Label, Margin, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use egui_phosphor::regular as ph;
use roncad_core::command::AppCommand;
use roncad_core::ids::WorkplaneId;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const TOOLBAR_HEIGHT: f32 = 40.0;
const TOOLBAR_ICON_SIZE: f32 = 24.0;

pub fn render(ui: &mut Ui, shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
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
                    new_sketch_plane_selector(ui, shell);
                    if icon_button(ui, ph::PLUS, "Create sketch").clicked() {
                        let plane = preferred_sketch_plane(shell).unwrap_or_else(|| {
                            shell.project.workplanes.keys().next().expect("plane")
                        });
                        response.commands.push(AppCommand::CreateSketch {
                            name: format!("Sketch {}", shell.project.sketches.len() + 1),
                            plane,
                        });
                    }

                    toolbar_divider(ui);
                    let _ = icon_button(ui, ph::FLOPPY_DISK, "Save project");
                    let _ = icon_button(ui, ph::ARROWS_COUNTER_CLOCKWISE, "Undo");
                    let _ = icon_button(ui, ph::ARROW_CLOCKWISE, "Redo");
                    if icon_button(ui, ph::PROJECTOR_SCREEN, "Fit view").clicked() {
                        response.fit_view_requested = true;
                    }

                    ui.push_id("toolbar_trailing", |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let _ = icon_button(ui, ph::GEAR, "Application settings");
                            mode_chip(
                                ui,
                                active_tool.label(),
                                tool_accent,
                                ThemeColors::tool_accent_dim(active_tool),
                            );
                            ui.add_sized(
                                Vec2::new(120.0, TOOLBAR_ICON_SIZE),
                                Label::new(
                                    RichText::new(&shell.project.name)
                                        .size(12.0)
                                        .color(ThemeColors::TEXT_DIM),
                                )
                                .truncate(),
                            );
                            let command_bar = command_bar_hint(
                                ui,
                                ui.ctx().os(),
                                shell.command_palette.is_open(),
                            )
                            .on_hover_text("Search tools, sketches, and commands");
                            if command_bar.clicked() {
                                shell.command_palette.open();
                            }
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
    let selected = shell.project.active_sketch().map_or_else(
        || "No Sketch".to_string(),
        |sketch| {
            let plane = shell
                .project
                .workplanes
                .get(sketch.workplane)
                .map(|plane| plane.name.as_str())
                .unwrap_or("?");
            format!("{} · {}", sketch.name, plane)
        },
    );

    ComboBox::from_id_salt("active_sketch_selector")
        .width(172.0)
        .selected_text(RichText::new(selected).size(12.0))
        .show_ui(ui, |ui| {
            for (id, sketch) in shell.project.sketches.iter() {
                let selected = shell.project.active_sketch == Some(id);
                let plane = shell
                    .project
                    .workplanes
                    .get(sketch.workplane)
                    .map(|plane| plane.name.as_str())
                    .unwrap_or("?");
                let label = format!("{} · {}", sketch.name, plane);
                if ui.selectable_label(selected, label).clicked() && !selected {
                    response.commands.push(AppCommand::SetActiveSketch(id));
                    ui.close();
                }
            }
        });
}

fn new_sketch_plane_selector(ui: &mut Ui, shell: &mut ShellContext<'_>) {
    let Some(selected_plane) = preferred_sketch_plane(shell) else {
        return;
    };
    let selected_label = shell
        .project
        .workplanes
        .get(selected_plane)
        .map(|plane| plane.name.as_str())
        .unwrap_or("Plane");

    ComboBox::from_id_salt("new_sketch_plane_selector")
        .width(72.0)
        .selected_text(RichText::new(selected_label).size(12.0))
        .show_ui(ui, |ui| {
            for (id, plane) in shell.project.workplanes.iter() {
                let selected = selected_plane == id;
                if ui.selectable_label(selected, &plane.name).clicked() && !selected {
                    *shell.new_sketch_plane = Some(id);
                    ui.close();
                }
            }
        });
}

fn preferred_sketch_plane(shell: &ShellContext<'_>) -> Option<WorkplaneId> {
    (*shell.new_sketch_plane)
        .or(shell.project.active_sketch().map(|sketch| sketch.workplane))
        .or_else(|| shell.project.workplanes.keys().next())
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

fn command_bar_hint(ui: &mut Ui, os: OperatingSystem, open: bool) -> egui::Response {
    let size = Vec2::new(156.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if open {
        ThemeColors::BG_HEADER_ACTIVE
    } else if response.hovered() {
        ThemeColors::BG_HOVER
    } else {
        ThemeColors::BG_PANEL_ALT
    };
    let stroke = if open {
        Stroke::new(1.0, ThemeColors::ACCENT_DIM)
    } else if response.hovered() {
        Stroke::new(1.0, ThemeColors::BG_ACTIVE)
    } else {
        Stroke::new(1.0, ThemeColors::SEPARATOR)
    };
    ui.painter().rect_filled(rect, 2.0, fill);
    ui.painter()
        .rect_stroke(rect, 2.0, stroke, StrokeKind::Outside);

    let text_color = if open {
        ThemeColors::TEXT
    } else if response.hovered() {
        ThemeColors::TEXT_MID
    } else {
        ThemeColors::TEXT_DIM
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        ph::MAGNIFYING_GLASS,
        FontId::proportional(11.5),
        text_color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(24.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "Search · run",
        FontId::proportional(11.0),
        text_color,
    );

    let shortcut_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 48.0, rect.top() + 3.0),
        egui::vec2(40.0, 18.0),
    );
    ui.painter()
        .rect_filled(shortcut_rect, 2.0, ThemeColors::BG_DEEP);
    ui.painter().rect_stroke(
        shortcut_rect,
        2.0,
        Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT),
        StrokeKind::Outside,
    );
    ui.painter().text(
        shortcut_rect.center(),
        egui::Align2::CENTER_CENTER,
        palette_shortcut_label(os),
        FontId::monospace(8.5),
        ThemeColors::TEXT_MID,
    );

    response
}

fn palette_shortcut_label(os: OperatingSystem) -> &'static str {
    match os {
        OperatingSystem::Mac => "⌘K",
        _ => "Ctrl K",
    }
}
