//! Left tool shelf. Tools are selected here; activation flips the manager
//! to the matching Tool implementation (which handles its own state reset).

use egui::{pos2, Align2, Button, FontId, Panel, Rect, RichText, Ui, Vec2};
use egui_phosphor::regular as ph;
use roncad_tools::ActiveToolKind;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const TOOLS: &[ActiveToolKind] = &[
    ActiveToolKind::Select,
    ActiveToolKind::Pan,
    ActiveToolKind::Line,
    ActiveToolKind::Rectangle,
    ActiveToolKind::Circle,
    ActiveToolKind::Arc,
    ActiveToolKind::Fillet,
    ActiveToolKind::Extrude,
    ActiveToolKind::Revolve,
];

pub fn render(ui: &mut Ui, shell: &mut ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::left("tool_shelf")
        .exact_size(52.0)
        .resizable(false)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.push_id("tool_shelf_column", |ui| {
                ui.vertical_centered(|ui| {
                    let current = shell.tool_manager.active_kind();
                    for tool in TOOLS {
                        ui.push_id(("tool_button", tool.label()), |ui| {
                            let active = current == *tool;
                            let accent = ThemeColors::tool_accent(*tool);
                            let label = RichText::new(tool_glyph(*tool))
                                .font(FontId::proportional(18.0))
                                .color(if active {
                                    accent
                                } else {
                                    ThemeColors::TEXT_DIM
                                });
                            let response = ui.add_sized(
                                Vec2::new(38.0, 38.0),
                                Button::selectable(active, label),
                            );
                            paint_tool_chrome(ui, response.rect, *tool, active, accent);
                            if response.clicked() {
                                shell.tool_manager.set_active(*tool);
                            }
                            response.on_hover_text(tool_hover_text(*tool));
                            ui.add_space(2.0);
                        });
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);
                    let _ = ui.add_sized(
                        Vec2::new(38.0, 38.0),
                        Button::new(
                            RichText::new(ph::GEAR)
                                .font(FontId::proportional(18.0))
                                .color(ThemeColors::TEXT_DIM),
                        ),
                    );
                });
            });
        });
}

fn tool_hover_text(tool: ActiveToolKind) -> String {
    match tool.shortcut() {
        Some(shortcut) => format!("{} ({shortcut})", tool.label()),
        None => tool.label().to_string(),
    }
}

fn tool_glyph(tool: ActiveToolKind) -> &'static str {
    match tool {
        ActiveToolKind::Select => ph::CURSOR,
        ActiveToolKind::Pan => ph::HAND,
        ActiveToolKind::Line => ph::LINE_SEGMENT,
        ActiveToolKind::Rectangle => ph::RECTANGLE,
        ActiveToolKind::Circle => ph::CIRCLE,
        ActiveToolKind::Arc => "A",
        ActiveToolKind::Fillet => "F",
        ActiveToolKind::Dimension => ph::RULER,
        ActiveToolKind::Extrude => ph::ARROW_FAT_LINE_UP,
        ActiveToolKind::Revolve => ph::ARROWS_CLOCKWISE,
    }
}

fn paint_tool_chrome(
    ui: &Ui,
    rect: Rect,
    tool: ActiveToolKind,
    active: bool,
    accent: egui::Color32,
) {
    if active {
        let rail = Rect::from_min_max(
            pos2(rect.left() - 7.0, rect.top() + 7.0),
            pos2(rect.left() - 5.0, rect.bottom() - 7.0),
        );
        ui.painter().rect_filled(rail, 1.0, accent);
    }

    if let Some(shortcut) = tool.shortcut() {
        let shortcut_color = if active {
            ThemeColors::tool_accent_dim(tool)
        } else {
            ThemeColors::TEXT_FAINT
        };
        ui.painter().text(
            rect.right_bottom() + egui::vec2(-3.0, -3.0),
            Align2::RIGHT_BOTTOM,
            shortcut,
            FontId::monospace(8.0),
            shortcut_color,
        );
    }
}
