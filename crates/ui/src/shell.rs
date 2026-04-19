//! Top-level panel layout: toolbar (top), tool shelf (left),
//! inspector + project tree (right), status bar (bottom), viewport (center).

use egui::{pos2, Rect, Sense, Ui};
use roncad_core::command::AppCommand;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, SnapResult, ToolManager};

use crate::HudEditState;
use crate::{
    command_palette, right_sidebar, status_bar, tool_shelf, toolbar,
    viewport::{self, ViewportInteractionHandler},
    CommandPaletteState,
};

pub struct ShellContext<'a> {
    pub tool_manager: &'a mut ToolManager,
    pub snap_engine: &'a SnapEngine,
    pub snap_result: &'a mut Option<SnapResult>,
    pub selection: &'a Selection,
    pub camera: &'a mut Camera2d,
    pub project: &'a Project,
    pub cursor_world_mm: &'a mut Option<glam::DVec2>,
    pub hud_state: &'a mut HudEditState,
    pub command_palette: &'a mut CommandPaletteState,
}

#[derive(Default)]
pub struct ShellResponse {
    pub commands: Vec<AppCommand>,
    pub quit_requested: bool,
}

pub fn render_shell(
    ui: &mut Ui,
    shell: &mut ShellContext<'_>,
    viewport_interaction: ViewportInteractionHandler,
) -> ShellResponse {
    let mut response = ShellResponse::default();
    command_palette::handle_shortcut(ui.ctx(), shell.command_palette);

    toolbar::render(ui, shell, &mut response);
    tool_shelf::render(ui, shell, &mut response);
    right_sidebar::render(ui, shell, &mut response);
    let remaining = ui.available_rect_before_wrap();
    if remaining.is_positive() {
        let _ = ui.allocate_rect(remaining, Sense::hover());
        let (viewport_rect, status_rect) = split_shell_rect(remaining, 24.0);
        viewport::render_in_rect(
            ui,
            viewport_rect,
            shell,
            &mut response,
            viewport_interaction,
        );
        status_bar::render_in_rect(ui, status_rect, shell, &mut response);
    }
    command_palette::render(ui.ctx(), shell, &mut response);

    response
}

fn split_shell_rect(rect: Rect, status_height: f32) -> (Rect, Rect) {
    let status_top = (rect.max.y - status_height).max(rect.min.y);
    let viewport_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, status_top));
    let status_rect = Rect::from_min_max(pos2(rect.min.x, status_top), rect.max);
    (viewport_rect, status_rect)
}
