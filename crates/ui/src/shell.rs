//! Top-level panel layout: toolbar (top), tool shelf (left),
//! inspector + project tree (right), status bar (bottom), viewport (center).

use egui::{pos2, Rect, Sense, Ui};
use roncad_core::command::AppCommand;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, SnapResult, ToolManager};

use crate::{
    command_palette, right_sidebar, status_bar, tool_shelf, toolbar,
    viewport::{self, ViewportInteractionHandler},
    CommandPaletteState,
};
use crate::{ExtrudeHudState, HudEditState};

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
    pub extrude_hud: &'a mut ExtrudeHudState,
}

#[derive(Default)]
pub struct ShellResponse {
    pub commands: Vec<AppCommand>,
    pub fit_view_requested: bool,
    pub quit_requested: bool,
}

pub fn render_shell(
    ui: &mut Ui,
    shell: &mut ShellContext<'_>,
    viewport_interaction: ViewportInteractionHandler,
) -> ShellResponse {
    let mut response = ShellResponse::default();
    command_palette::handle_shortcut(ui.ctx(), shell.command_palette);
    shell
        .extrude_hud
        .sync_active_tool(shell.tool_manager.active_kind());

    toolbar::render(ui, shell, &mut response);
    tool_shelf::render(ui, shell, &mut response);
    right_sidebar::render(ui, shell, &mut response);
    let mut viewport_rect = None;
    let remaining = ui.available_rect_before_wrap();
    if remaining.is_positive() {
        let _ = ui.allocate_rect(remaining, Sense::hover());
        let (viewport_rect_inner, status_rect) = split_shell_rect(remaining, 24.0);
        viewport_rect = Some(viewport_rect_inner);
        viewport::render_in_rect(
            ui,
            viewport_rect_inner,
            shell,
            &mut response,
            viewport_interaction,
        );
        status_bar::render_in_rect(ui, status_rect, shell, &mut response);
    }
    command_palette::render(ui.ctx(), shell, &mut response);
    if response.fit_view_requested {
        if let Some(rect) = viewport_rect {
            fit_active_view(shell, rect);
            ui.ctx().request_repaint();
        }
    }

    response
}

fn split_shell_rect(rect: Rect, status_height: f32) -> (Rect, Rect) {
    let status_top = (rect.max.y - status_height).max(rect.min.y);
    let viewport_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, status_top));
    let status_rect = Rect::from_min_max(pos2(rect.min.x, status_top), rect.max);
    (viewport_rect, status_rect)
}

fn fit_active_view(shell: &mut ShellContext<'_>, rect: Rect) {
    let Some(sketch) = shell.project.active_sketch() else {
        return;
    };
    let Some((min, max)) = sketch.bounds() else {
        return;
    };

    shell.camera.fit_bounds(
        glam::DVec2::new(rect.width() as f64, rect.height() as f64),
        min,
        max,
        36.0,
    );
}
