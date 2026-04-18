use egui::{Pos2, Rect, Ui};

use crate::shell::ShellContext;
use crate::theme::ThemeColors;

pub(super) fn paint(ui: &mut Ui, rect: Rect, shell: &ShellContext<'_>) {
    let painter = ui.painter_at(rect);
    let pad = 8.0;
    let kind = shell.tool_manager.active_kind();
    let text = format!(
        "{}   |   {}   |   middle/right-drag: pan   |   scroll: zoom",
        kind.label(),
        kind.hint()
    );
    painter.text(
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(12.0),
        ThemeColors::TEXT_DIM,
    );
}
