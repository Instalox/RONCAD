use egui::{Color32, Pos2, Rect, Ui};
use roncad_core::ids::{SketchEntityId, SketchId};

use crate::dimensions;
use crate::shell::ShellContext;
use crate::theme::ThemeColors;

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &ShellContext<'_>,
    hovered_entity: Option<(SketchId, SketchEntityId)>,
) {
    let painter = ui.painter_at(rect);
    let pad = 8.0;
    let kind = shell.tool_manager.active_kind();
    let accent = ThemeColors::tool_accent(kind);
    let shortcut = kind.shortcut().map_or(String::new(), |shortcut| format!(" [{shortcut}]"));
    let text = format!(
        "{}{}   |   {}   |   middle/right-drag: pan   |   scroll: zoom",
        kind.label(),
        shortcut,
        shell.tool_manager.step_hint(),
    );
    painter.text(
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(12.0),
        accent.gamma_multiply(0.85),
    );

    if let Some(hover_text) = dimensions::hovered_entity_summary(shell.project, hovered_entity) {
        let font = egui::FontId::monospace(11.0);
        let shadow = Pos2::new(rect.min.x + pad + 1.0, rect.min.y + pad + 21.0);
        let anchor = Pos2::new(rect.min.x + pad, rect.min.y + pad + 20.0);
        painter.text(
            shadow,
            egui::Align2::LEFT_TOP,
            &hover_text,
            font.clone(),
            Color32::BLACK,
        );
        painter.text(
            anchor,
            egui::Align2::LEFT_TOP,
            hover_text,
            font,
            ThemeColors::ACCENT_AMBER.gamma_multiply(0.92),
        );
    }
}
