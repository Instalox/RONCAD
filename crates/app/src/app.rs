//! RonCadApp is the composition root: it owns the document, UI, tool,
//! and render-cache state buckets and drives the per-frame update.

use eframe::{App, CreationContext, Frame};
use egui::Ui;
use roncad_core::command::AppCommand;
use roncad_core::ids::WorkplaneId;
use roncad_core::selection::Selection;
use roncad_geometry::Project;
use roncad_rendering::Camera2d;
use roncad_tools::{SnapEngine, SnapResult, ToolManager};
use roncad_ui::{
    apply_dark_theme, render_shell, CommandPaletteState, ExtrudeHudState, HudEditState,
    RevolveHudState, ShellContext,
};

use crate::dispatcher;
use crate::interaction_controller;

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

#[derive(Default)]
struct UiState {
    hud_state: HudEditState,
    command_palette: CommandPaletteState,
    extrude_hud: ExtrudeHudState,
    revolve_hud: RevolveHudState,
    new_sketch_plane: Option<WorkplaneId>,
}

pub struct RonCadApp {
    document: DocumentState,
    tool: ToolRuntimeState,
    render: RenderCache,
    ui: UiState,
}

impl RonCadApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        let mut app = Self {
            document: DocumentState::default(),
            tool: ToolRuntimeState::default(),
            render: RenderCache::default(),
            ui: UiState::default(),
        };
        app.align_camera_to_active_sketch();
        app
    }

    fn dispatch(&mut self, commands: Vec<AppCommand>) {
        let previous_sketch = self.document.project.active_sketch;
        for cmd in commands {
            tracing::debug!(?cmd, "apply");
            dispatcher::apply(
                &mut self.document.project,
                &mut self.document.selection,
                &cmd,
            );
        }
        if self.document.project.active_sketch != previous_sketch {
            self.align_camera_to_active_sketch();
        }
        if self.ui.new_sketch_plane.is_none() {
            self.ui.new_sketch_plane = self
                .document
                .project
                .active_sketch()
                .map(|sketch| sketch.workplane);
        }
    }

    fn align_camera_to_active_sketch(&mut self) {
        let Some(sketch_id) = self.document.project.active_sketch else {
            return;
        };
        let Some(sketch) = self.document.project.sketches.get(sketch_id) else {
            return;
        };
        let Some(workplane) = self.document.project.workplanes.get(sketch.workplane) else {
            return;
        };

        self.render.camera.align_to_workplane(workplane);
        self.ui.new_sketch_plane = Some(sketch.workplane);

        let (min, max) = if let Some((local_min, local_max)) = sketch.bounds() {
            workplane.local_bounds_to_world_bounds(
                glam::DVec3::new(local_min.x, local_min.y, 0.0),
                glam::DVec3::new(local_max.x, local_max.y, 0.0),
            )
        } else {
            workplane.local_bounds_to_world_bounds(
                glam::DVec3::new(-40.0, -40.0, 0.0),
                glam::DVec3::new(40.0, 40.0, 0.0),
            )
        };

        self.render
            .camera
            .fit_bounds_3d(self.render.camera.viewport_size_px(), min, max, 56.0);
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
            hud_state: &mut self.ui.hud_state,
            command_palette: &mut self.ui.command_palette,
            extrude_hud: &mut self.ui.extrude_hud,
            revolve_hud: &mut self.ui.revolve_hud,
            new_sketch_plane: &mut self.ui.new_sketch_plane,
        };

        let response = render_shell(
            ui,
            &mut shell,
            interaction_controller::handle_viewport_interaction,
        );

        self.dispatch(response.commands);
        if response.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
