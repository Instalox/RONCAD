//! Left tool shelf. Tools are selected here; activation flips the manager
//! to the matching Tool implementation (which handles its own state reset).

use egui::{Button, FontId, Panel, RichText, Ui, Vec2};
use egui_phosphor::regular as ph;
use roncad_tools::ActiveToolKind;

use crate::shell::{ShellContext, ShellResponse};

const TOOLS: &[ActiveToolKind] = &[
    ActiveToolKind::Select,
    ActiveToolKind::Pan,
    ActiveToolKind::Line,
    ActiveToolKind::Rectangle,
    ActiveToolKind::Circle,
    ActiveToolKind::Dimension,
    ActiveToolKind::Extrude,
];

pub fn render(ui: &mut Ui, shell: &mut ShellContext<'_>, _response: &mut ShellResponse) {
    Panel::left("tool_shelf")
        .exact_size(56.0)
        .resizable(false)
        .show_inside(ui, |ui| {
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                let current = shell.tool_manager.active_kind();
                for tool in TOOLS {
                    let active = current == *tool;
                    let label = RichText::new(tool_glyph(*tool))
                        .font(FontId::proportional(20.0));
                    let response = ui.add_sized(
                        Vec2::new(44.0, 44.0),
                        Button::selectable(active, label),
                    );
                    if response.clicked() {
                        shell.tool_manager.set_active(*tool);
                    }
                    response.on_hover_text(tool.label());
                    ui.add_space(2.0);
                }
            });
        });
}

fn tool_glyph(tool: ActiveToolKind) -> &'static str {
    match tool {
        ActiveToolKind::Select => ph::CURSOR,
        ActiveToolKind::Pan => ph::HAND,
        ActiveToolKind::Line => ph::LINE_SEGMENT,
        ActiveToolKind::Rectangle => ph::RECTANGLE,
        ActiveToolKind::Circle => ph::CIRCLE,
        ActiveToolKind::Dimension => ph::RULER,
        ActiveToolKind::Extrude => ph::ARROW_FAT_LINE_UP,
    }
}
