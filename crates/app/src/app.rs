//! RonCadApp is the composition root: it owns the document, UI, tool,
//! and render-cache state buckets and drives the per-frame update.

use eframe::{App, CreationContext, Frame};
use egui::Ui;
use roncad_core::command::AppCommand;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, SnapResult, ToolManager};
use roncad_ui::{apply_dark_theme, render_shell, ShellContext};

use crate::dispatcher;

struct DocumentState {
    project: Project,
    selection: Selection,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            project: Project::new_untitled(),
            selection: Selection::default(),
        }
    }
}

#[derive(Default)]
struct ToolRuntimeState {
    manager: ToolManager,
    snap_engine: SnapEngine,
    snap_result: Option<SnapResult>,
    cursor_world_mm: Option<glam::DVec2>,
}

#[derive(Default)]
struct RenderCache {
    camera: Camera2d,
}

pub struct RonCadApp {
    document: DocumentState,
    tool: ToolRuntimeState,
    render: RenderCache,
}

impl RonCadApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        Self {
            document: DocumentState::default(),
            tool: ToolRuntimeState::default(),
            render: RenderCache::default(),
        }
    }

    fn dispatch(&mut self, commands: Vec<AppCommand>) {
        for cmd in commands {
            tracing::debug!(?cmd, "apply");
            dispatcher::apply(
                &mut self.document.project,
                &mut self.document.selection,
                &cmd,
            );
        }
    }
}

impl App for RonCadApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let mut shell = ShellContext {
            tool_manager: &mut self.tool.manager,
            snap_engine: &self.tool.snap_engine,
            snap_result: &mut self.tool.snap_result,
            selection: &self.document.selection,
            camera: &mut self.render.camera,
            project: &self.document.project,
            cursor_world_mm: &mut self.tool.cursor_world_mm,
        };

        let response = render_shell(ui, &mut shell);

        self.dispatch(response.commands);
        if response.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
