//! Top-level panel layout: toolbar (top), tool shelf (left),
//! inspector + project tree (right), status bar (bottom), viewport (center).

use egui::Ui;
use roncad_core::command::AppCommand;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, ToolManager};

use crate::{inspector, project_tree, status_bar, tool_shelf, toolbar, viewport};

pub struct ShellContext<'a> {
    pub tool_manager: &'a mut ToolManager,
    pub snap_engine: &'a SnapEngine,
    pub selection: &'a Selection,
    pub camera: &'a mut Camera2d,
    pub project: &'a Project,
    pub cursor_world_mm: &'a mut Option<glam::DVec2>,
}

#[derive(Default)]
pub struct ShellResponse {
    pub commands: Vec<AppCommand>,
    pub quit_requested: bool,
}

pub fn render_shell(ui: &mut Ui, shell: &mut ShellContext<'_>) -> ShellResponse {
    let mut response = ShellResponse::default();

    toolbar::render(ui, shell, &mut response);
    tool_shelf::render(ui, shell, &mut response);
    project_tree::render(ui, shell, &mut response);
    inspector::render(ui, shell, &mut response);
    status_bar::render(ui, shell, &mut response);
    viewport::render(ui, shell, &mut response);

    response
}
