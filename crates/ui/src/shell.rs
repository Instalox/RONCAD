//! Top-level panel layout: toolbar (top), tool shelf (left),
//! inspector + project tree (right), status bar (bottom), viewport (center).

use std::path::{Path, PathBuf};

use egui::{pos2, Rect, Sense, Ui};
use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::constraint::EntityPoint;
use roncad_core::ids::{ConstraintId, SketchEntityId, SketchId, WorkplaneId};
use roncad_core::selection::Selection;
use roncad_geometry::{Project, SolveReport};
use roncad_rendering::Camera2d;
use roncad_tools::{PreselectionState, SnapEngine, SnapResult, ToolManager};

use crate::{
    command_palette,
    constraints::ConstraintPanelState,
    right_sidebar, status_bar, tool_shelf, toolbar,
    viewport::{self, ViewportInteractionHandler},
    CommandPaletteState,
};
use crate::{ExtrudeHudState, HudEditState, RevolveHudState};

pub struct ShellContext<'a> {
    pub tool_manager: &'a mut ToolManager,
    pub snap_engine: &'a SnapEngine,
    pub snap_result: &'a mut Option<SnapResult>,
    pub selection: &'a Selection,
    pub camera: &'a mut Camera2d,
    pub project: &'a Project,
    pub cursor_world_mm: &'a mut Option<glam::DVec2>,
    pub preselection: &'a mut PreselectionState,
    pub selection_move: &'a mut SelectionMoveState,
    pub hud_state: &'a mut HudEditState,
    pub command_palette: &'a mut CommandPaletteState,
    pub extrude_hud: &'a mut ExtrudeHudState,
    pub new_sketch_plane: &'a mut Option<WorkplaneId>,
    pub revolve_hud: &'a mut RevolveHudState,
    pub constraint_panel: &'a mut ConstraintPanelState,
    pub document_dirty: bool,
    pub document_path: Option<&'a Path>,
    pub recent_files: &'a [PathBuf],
    pub status_text: Option<&'a str>,
    pub status_is_error: bool,
    pub last_solve_report: Option<&'a SolveReport>,
}

#[derive(Debug, Clone, Default)]
pub struct SelectionMoveState {
    pub drag: Option<SelectionMoveDrag>,
}

#[derive(Debug, Clone)]
pub struct SelectionMoveDrag {
    pub sketch: SketchId,
    pub anchor: DVec2,
    pub current: DVec2,
    pub vertices: Vec<EntityPoint>,
    pub entities: Vec<SketchEntityId>,
}

impl SelectionMoveState {
    pub fn clear(&mut self) {
        self.drag = None;
    }

    pub fn begin(
        &mut self,
        sketch: SketchId,
        anchor: DVec2,
        vertices: Vec<EntityPoint>,
        entities: Vec<SketchEntityId>,
    ) {
        self.drag = Some(SelectionMoveDrag {
            sketch,
            anchor,
            current: anchor,
            vertices,
            entities,
        });
    }

    pub fn update(&mut self, current: DVec2) {
        if let Some(drag) = self.drag.as_mut() {
            drag.current = current;
        }
    }

    pub fn finish(&mut self) -> Option<SelectionMoveDrag> {
        self.drag.take()
    }

    pub fn delta(&self) -> Option<DVec2> {
        self.drag.as_ref().map(SelectionMoveDrag::delta)
    }

    pub fn is_active(&self) -> bool {
        self.drag.is_some()
    }
}

impl SelectionMoveDrag {
    pub fn delta(&self) -> DVec2 {
        self.current - self.anchor
    }
}

#[derive(Default)]
pub struct ShellResponse {
    pub commands: Vec<AppCommand>,
    pub highlighted_sketch_entities: Vec<(SketchId, SketchEntityId)>,
    pub highlighted_constraint: Option<(SketchId, ConstraintId)>,
    pub fit_view_requested: bool,
    pub fit_selection_requested: bool,
    pub quit_requested: bool,
    pub open_project_requested: bool,
    pub open_project_by_path_requested: bool,
    pub open_project_path: Option<PathBuf>,
    pub save_project_requested: bool,
    pub save_project_as_requested: bool,
    pub save_project_as_path_requested: bool,
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
    if response.fit_view_requested || response.fit_selection_requested {
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
    let Some((min, max)) = fit_bounds(shell.project) else {
        return;
    };

    shell.camera.fit_bounds_3d(
        glam::DVec2::new(rect.width() as f64, rect.height() as f64),
        min,
        max,
        36.0,
    );
}

fn fit_bounds(project: &Project) -> Option<(glam::DVec3, glam::DVec3)> {
    let mut min = glam::DVec3::splat(f64::INFINITY);
    let mut max = glam::DVec3::splat(f64::NEG_INFINITY);
    let mut found = false;

    if let Some(sketch) = project.active_sketch() {
        if let Some((sketch_min, sketch_max)) = sketch.bounds() {
            if let Some(workplane) = project.active_workplane() {
                let (world_min, world_max) = workplane.local_bounds_to_world_bounds(
                    glam::DVec3::new(sketch_min.x, sketch_min.y, 0.0),
                    glam::DVec3::new(sketch_max.x, sketch_max.y, 0.0),
                );
                min = min.min(world_min);
                max = max.max(world_max);
                found = true;
            }
        }
    }

    for (_, feature) in project.features.iter() {
        if let Some((feature_min, feature_max)) = project.feature_world_bounds(feature) {
            min = min.min(feature_min);
            max = max.max(feature_max);
            found = true;
        }
    }

    found.then_some((min, max))
}
