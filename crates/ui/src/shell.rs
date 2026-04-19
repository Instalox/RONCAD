//! Top-level panel layout: toolbar (top), tool shelf (left),
//! inspector + project tree (right), status bar (bottom), viewport (center).

use egui::Ui;
use roncad_core::command::AppCommand;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, SnapResult, ToolManager};

use crate::HudEditState;
use crate::{
    right_sidebar, status_bar, tool_shelf, toolbar,
    viewport::{self, ViewportInteractionHandler},
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

    toolbar::render(ui, shell, &mut response);
    tool_shelf::render(ui, shell, &mut response);
    right_sidebar::render(ui, shell, &mut response);
    status_bar::render(ui, shell, &mut response);
    viewport::render(ui, shell, &mut response, viewport_interaction);

    response
}
