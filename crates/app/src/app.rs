//! RonCadApp is the composition root: it owns the document, UI, tool,
//! and render-cache state buckets and drives the per-frame update.

use std::env;
use std::path::{Path, PathBuf};

use eframe::{App, CreationContext, Frame};
use egui::{vec2, Align2, Id, Key, Layout, Modal, Modifiers, RichText, TextEdit, Ui};
use roncad_core::command::AppCommand;
use roncad_core::ids::WorkplaneId;
use roncad_core::selection::Selection;
use roncad_geometry::{Project, SolveReport};
use roncad_project_io::{load_project, save_project};
use roncad_rendering::Camera2d;
use roncad_tools::{ActiveToolKind, PreselectionState, SnapEngine, SnapResult, ToolManager};
use roncad_ui::{
    apply_dark_theme, render_shell, theme::ThemeColors,
    viewport::wgpu_renderer::BodyRenderResources, CommandPaletteState, ConstraintPanelState,
    ExtrudeHudState, HudEditState, RevolveHudState, ShellContext, ShellResponse,
};

use crate::dispatcher;
use crate::interaction_controller;
use crate::settings::{load_app_settings, save_app_settings, AppSettings};

const MAX_RECENT_FILES: usize = 8;

struct DocumentState {
    project: Project,
    selection: Selection,
    path: Option<PathBuf>,
    dirty: bool,
    status_message: Option<StatusMessage>,
    last_solve_report: Option<SolveReport>,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            project: Project::new_untitled(),
            selection: Selection::default(),
            path: None,
            dirty: false,
            status_message: None,
            last_solve_report: None,
        }
    }
}

struct StatusMessage {
    text: String,
    is_error: bool,
}

#[derive(Default)]
struct ToolRuntimeState {
    manager: ToolManager,
    snap_engine: SnapEngine,
    snap_result: Option<SnapResult>,
    cursor_world_mm: Option<glam::DVec2>,
    preselection: PreselectionState,
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
    constraint_panel: ConstraintPanelState,
    new_sketch_plane: Option<WorkplaneId>,
    recent_files: Vec<PathBuf>,
    file_dialog: Option<FileDialogState>,
    discard_changes_dialog: Option<DiscardChangesDialogState>,
    allow_dirty_close_once: bool,
    last_viewport_title: Option<String>,
}

struct FileDialogState {
    mode: FileDialogMode,
    path_text: String,
    error: Option<String>,
    request_focus: bool,
    follow_up: Option<PendingDocumentAction>,
}

impl FileDialogState {
    fn new(
        mode: FileDialogMode,
        path_text: String,
        follow_up: Option<PendingDocumentAction>,
    ) -> Self {
        Self {
            mode,
            path_text,
            error: None,
            request_focus: true,
            follow_up,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileDialogMode {
    Open,
    SaveAs,
}

struct DiscardChangesDialogState {
    action: PendingDocumentAction,
    error: Option<String>,
}

impl DiscardChangesDialogState {
    fn new(action: PendingDocumentAction) -> Self {
        Self {
            action,
            error: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum PendingDocumentAction {
    OpenProject,
    OpenPath(PathBuf),
    Quit,
}

impl PendingDocumentAction {
    fn heading(&self) -> &'static str {
        match self {
            Self::OpenProject | Self::OpenPath(_) => "Open Another Project?",
            Self::Quit => "Quit RONCAD?",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::OpenProject | Self::OpenPath(_) => {
                "Save changes to the current project before opening a different file."
            }
            Self::Quit => "Save changes to the current project before closing the app.",
        }
    }
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

        // Register the wgpu body renderer with the egui_wgpu callback resource
        // map so paint callbacks can pull the cached pipelines & buffers.
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            let resources =
                BodyRenderResources::new(&render_state.device, render_state.target_format);
            render_state
                .renderer
                .write()
                .callback_resources
                .insert(resources);
        } else {
            tracing::error!("wgpu_render_state missing — viewport 3D rendering will be disabled");
        }

        let mut app = Self {
            document: DocumentState::default(),
            tool: ToolRuntimeState::default(),
            render: RenderCache::default(),
            ui: UiState::default(),
        };

        let mut restored_project = false;
        match load_app_settings() {
            Ok(settings) => {
                app.ui.recent_files = dedupe_recent_files(settings.recent_files);
                if let Some(path) = settings.last_project {
                    restored_project = app.restore_last_project(path);
                }
            }
            Err(error) => {
                app.document.status_message = Some(StatusMessage {
                    text: format!("Could not load app settings: {error:#}"),
                    is_error: true,
                });
            }
        }

        if !restored_project {
            app.align_camera_to_active_sketch();
        }
        app
    }

    fn dispatch(&mut self, commands: Vec<AppCommand>) {
        let previous_sketch = self.document.project.active_sketch;
        let mut last_solve_report = None;
        let mut document_changed = false;

        for command in commands {
            tracing::debug!(?command, "apply");
            if command_mutates_document(&command) {
                document_changed = true;
            }
            let solve_report = dispatcher::apply(
                &mut self.document.project,
                &mut self.document.selection,
                &command,
            );
            if solve_report.is_some() {
                last_solve_report = solve_report;
            }
        }

        if self.document.project.active_sketch != previous_sketch {
            self.align_camera_to_active_sketch();
        }
        if document_changed {
            self.document.dirty = true;
            if self
                .document
                .status_message
                .as_ref()
                .is_some_and(|message| !message.is_error)
            {
                self.document.status_message = None;
            }
        }
        if let Some(report) = last_solve_report {
            self.document.last_solve_report = Some(report);
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

    fn handle_global_shortcuts(&mut self, ui: &Ui, response: &mut ShellResponse) {
        if self.ui.command_palette.is_open()
            || self.ui.file_dialog.is_some()
            || self.ui.discard_changes_dialog.is_some()
        {
            return;
        }

        if ui.ctx().input_mut(|input| {
            input.consume_key(
                Modifiers {
                    command: true,
                    shift: true,
                    ..Default::default()
                },
                Key::S,
            )
        }) {
            response.save_project_as_requested = true;
            return;
        }

        if ui
            .ctx()
            .input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::S))
        {
            response.save_project_requested = true;
        }
        if ui
            .ctx()
            .input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::O))
        {
            response.open_project_requested = true;
        }
    }

    fn handle_shell_actions(&mut self, ui: &Ui, response: &ShellResponse) {
        if let Some(path) = response.open_project_path.clone() {
            self.request_document_action(PendingDocumentAction::OpenPath(path), ui);
        } else if response.open_project_requested {
            self.request_document_action(PendingDocumentAction::OpenProject, ui);
        } else if response.open_project_by_path_requested {
            self.open_file_dialog(FileDialogMode::Open, None);
        }
        if response.save_project_as_requested {
            self.save_project_as_with_picker();
        } else if response.save_project_as_path_requested {
            self.open_file_dialog(FileDialogMode::SaveAs, None);
        } else if response.save_project_requested {
            if let Some(path) = self.document.path.clone() {
                if let Err(error) = self.save_to_path(path) {
                    self.document.status_message = Some(StatusMessage {
                        text: error,
                        is_error: true,
                    });
                }
            } else {
                self.save_project_as_with_picker();
            }
        }
        if response.quit_requested {
            self.request_document_action(PendingDocumentAction::Quit, ui);
        }
    }

    fn request_document_action(&mut self, action: PendingDocumentAction, ui: &Ui) {
        if self.document.dirty {
            if matches!(action, PendingDocumentAction::Quit) {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            self.ui.discard_changes_dialog = Some(DiscardChangesDialogState::new(action));
        } else {
            self.execute_document_action(action, ui);
        }
    }

    fn execute_document_action(&mut self, action: PendingDocumentAction, ui: &Ui) {
        match action {
            PendingDocumentAction::OpenProject => {
                if let Some(path) = self.pick_project_path() {
                    self.load_project_path(path);
                }
            }
            PendingDocumentAction::OpenPath(path) => self.load_project_path(path),
            PendingDocumentAction::Quit => {
                if self.document.dirty {
                    self.ui.allow_dirty_close_once = true;
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn open_file_dialog(&mut self, mode: FileDialogMode, follow_up: Option<PendingDocumentAction>) {
        self.ui.file_dialog = Some(FileDialogState::new(
            mode,
            suggested_project_path(&self.document.project.name, self.document.path.as_deref())
                .display()
                .to_string(),
            follow_up,
        ));
    }

    fn project_file_dialog(&self) -> rfd::FileDialog {
        let suggested =
            suggested_project_path(&self.document.project.name, self.document.path.as_deref());
        let mut dialog = rfd::FileDialog::new().add_filter("RONCAD Project", &["json"]);
        if let Some(parent) = suggested.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(file_name) = suggested.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(file_name);
        }
        dialog
    }

    fn pick_project_path(&self) -> Option<PathBuf> {
        self.project_file_dialog().pick_file()
    }

    fn save_project_as_with_picker(&mut self) {
        let Some(path) = self
            .project_file_dialog()
            .save_file()
            .map(normalize_project_save_path)
        else {
            return;
        };

        if let Err(error) = self.save_to_path(path) {
            self.document.status_message = Some(StatusMessage {
                text: error,
                is_error: true,
            });
        }
    }

    fn render_file_dialog(&mut self, ui: &Ui) {
        let Some(dialog) = self.ui.file_dialog.as_mut() else {
            return;
        };

        let modal_id = Id::new("project_file_modal");
        let mut submit = false;
        let mut cancel = false;
        let modal = Modal::new(modal_id)
            .area(Modal::default_area(modal_id).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)));

        modal.show(ui.ctx(), |ui| {
            ui.set_width(520.0);

            if dialog.request_focus {
                ui.ctx()
                    .memory_mut(|memory| memory.request_focus(project_path_input_id()));
                dialog.request_focus = false;
            }

            ui.heading(match dialog.mode {
                FileDialogMode::Open => "Open Project",
                FileDialogMode::SaveAs => "Save Project As",
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Project file path").size(11.5));
            let input = ui.add(
                TextEdit::singleline(&mut dialog.path_text)
                    .id(project_path_input_id())
                    .desired_width(f32::INFINITY)
                    .hint_text("/path/to/project.roncad.json"),
            );

            if input.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                submit = true;
            }
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                cancel = true;
            }

            if let Some(error) = dialog.error.as_deref() {
                ui.add_space(6.0);
                ui.colored_label(egui::Color32::from_rgb(0xE2, 0xA2, 0x46), error);
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui
                    .button(match dialog.mode {
                        FileDialogMode::Open => "Open",
                        FileDialogMode::SaveAs => "Save",
                    })
                    .clicked()
                {
                    submit = true;
                }
            });
        });

        if cancel {
            self.ui.file_dialog = None;
        } else if submit {
            self.submit_file_dialog(ui);
        }
    }

    fn submit_file_dialog(&mut self, ui: &Ui) {
        let Some(mut dialog) = self.ui.file_dialog.take() else {
            return;
        };

        let path_text = dialog.path_text.trim();
        if path_text.is_empty() {
            dialog.error = Some("Path is required.".to_string());
            self.ui.file_dialog = Some(dialog);
            return;
        }

        let path = PathBuf::from(path_text);
        let result = match dialog.mode {
            FileDialogMode::Open => self.load_from_path(path),
            FileDialogMode::SaveAs => self.save_to_path(normalize_project_save_path(path)),
        };

        if let Err(error) = result {
            dialog.error = Some(error);
            self.ui.file_dialog = Some(dialog);
        } else if let Some(action) = dialog.follow_up {
            self.execute_document_action(action, ui);
        }
    }

    fn render_discard_changes_dialog(&mut self, ui: &Ui) {
        let Some(dialog) = self.ui.discard_changes_dialog.as_ref() else {
            return;
        };

        let modal_id = Id::new("discard_changes_modal");
        let mut save = false;
        let mut discard = false;
        let mut cancel = false;
        let action = dialog.action.clone();
        let error = dialog.error.clone();
        let modal = Modal::new(modal_id)
            .area(Modal::default_area(modal_id).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)));

        modal.show(ui.ctx(), |ui| {
            ui.set_width(420.0);

            if ui.input(|input| input.key_pressed(Key::Enter)) {
                save = true;
            }
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                cancel = true;
            }

            ui.heading(action.heading());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.colored_label(ThemeColors::ACCENT_AMBER, RichText::new("●").size(12.0));
                ui.label(RichText::new("Unsaved changes").strong());
            });
            ui.add_space(6.0);
            ui.label(action.description());
            ui.add_space(6.0);
            ui.monospace(document_label(
                &self.document.project.name,
                self.document.path.as_deref(),
            ));

            if let Some(error) = error.as_deref() {
                ui.add_space(8.0);
                ui.colored_label(ThemeColors::ACCENT_AMBER, error);
            }

            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([82.0, 24.0], egui::Button::new("Save"))
                    .clicked()
                {
                    save = true;
                }
                if ui
                    .add_sized([96.0, 24.0], egui::Button::new("Don't Save"))
                    .clicked()
                {
                    discard = true;
                }
                if ui
                    .add_sized([82.0, 24.0], egui::Button::new("Cancel"))
                    .clicked()
                {
                    cancel = true;
                }
            });
        });

        if cancel {
            self.ui.discard_changes_dialog = None;
            return;
        }

        if discard {
            self.ui.discard_changes_dialog = None;
            self.execute_document_action(action, ui);
            return;
        }

        if save {
            if let Some(path) = self.document.path.clone() {
                match self.save_to_path(path) {
                    Ok(()) => {
                        self.ui.discard_changes_dialog = None;
                        self.execute_document_action(action, ui);
                    }
                    Err(error) => {
                        if let Some(dialog) = self.ui.discard_changes_dialog.as_mut() {
                            dialog.error = Some(error);
                        }
                    }
                }
            } else {
                self.ui.discard_changes_dialog = None;
                self.open_file_dialog(FileDialogMode::SaveAs, Some(action));
            }
        }
    }

    fn handle_window_close_request(&mut self, ui: &Ui) {
        if !ui.ctx().input(|input| input.viewport().close_requested()) {
            return;
        }

        if self.ui.allow_dirty_close_once {
            self.ui.allow_dirty_close_once = false;
            return;
        }

        if !self.document.dirty {
            return;
        }

        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::CancelClose);
        if self.ui.file_dialog.is_none() && self.ui.discard_changes_dialog.is_none() {
            self.ui.discard_changes_dialog =
                Some(DiscardChangesDialogState::new(PendingDocumentAction::Quit));
        }
    }

    fn sync_viewport_title(&mut self, ui: &Ui) {
        let title = viewport_title(
            &self.document.project.name,
            self.document.path.as_deref(),
            self.document.dirty,
        );

        if self.ui.last_viewport_title.as_deref() == Some(title.as_str()) {
            return;
        }

        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        self.ui.last_viewport_title = Some(title);
    }

    fn load_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        let project = load_project(&path).map_err(|error| error.to_string())?;
        self.document.project = project;
        self.document.selection.clear();
        self.document.path = Some(path.clone());
        self.document.dirty = false;
        self.document.last_solve_report = None;
        self.document.status_message = Some(StatusMessage {
            text: format!("Loaded {}", path.display()),
            is_error: false,
        });
        self.tool.manager.set_active(ActiveToolKind::Select);
        self.tool.snap_result = None;
        self.tool.cursor_world_mm = None;
        self.tool.preselection.clear();
        self.ui.hud_state.clear();
        self.ui.command_palette.close();
        self.ui.extrude_hud.clear();
        self.ui.revolve_hud.clear();
        self.record_recent_file(&path);
        self.align_camera_to_active_sketch();
        Ok(())
    }

    fn save_to_path(&mut self, path: PathBuf) -> Result<(), String> {
        save_project(&self.document.project, &path).map_err(|error| error.to_string())?;
        self.document.path = Some(path.clone());
        self.document.dirty = false;
        self.document.status_message = Some(StatusMessage {
            text: format!("Saved {}", path.display()),
            is_error: false,
        });
        self.record_recent_file(&path);
        Ok(())
    }

    fn load_project_path(&mut self, path: PathBuf) {
        if let Err(error) = self.load_from_path(path) {
            self.document.status_message = Some(StatusMessage {
                text: error,
                is_error: true,
            });
        }
    }

    fn record_recent_file(&mut self, path: &Path) {
        push_recent_file(&mut self.ui.recent_files, path.to_path_buf());
        if let Err(error) = self.persist_app_settings() {
            tracing::warn!(error = %error, "failed to persist app settings");
        }
    }

    fn current_app_settings(&self) -> AppSettings {
        AppSettings {
            recent_files: self.ui.recent_files.clone(),
            last_project: self.document.path.clone(),
        }
    }

    fn persist_app_settings(&self) -> anyhow::Result<()> {
        save_app_settings(&self.current_app_settings())
    }

    fn restore_last_project(&mut self, path: PathBuf) -> bool {
        match self.load_from_path(path.clone()) {
            Ok(()) => true,
            Err(error) => {
                self.document.status_message = Some(StatusMessage {
                    text: format!("Could not reopen last project {}: {error}", path.display()),
                    is_error: true,
                });
                if let Err(settings_error) = self.persist_app_settings() {
                    tracing::warn!(error = %settings_error, "failed to update app settings");
                }
                false
            }
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
            preselection: &mut self.tool.preselection,
            hud_state: &mut self.ui.hud_state,
            command_palette: &mut self.ui.command_palette,
            extrude_hud: &mut self.ui.extrude_hud,
            revolve_hud: &mut self.ui.revolve_hud,
            constraint_panel: &mut self.ui.constraint_panel,
            new_sketch_plane: &mut self.ui.new_sketch_plane,
            document_dirty: self.document.dirty,
            document_path: self.document.path.as_deref(),
            recent_files: &self.ui.recent_files,
            status_text: self
                .document
                .status_message
                .as_ref()
                .map(|message| message.text.as_str()),
            status_is_error: self
                .document
                .status_message
                .as_ref()
                .is_some_and(|message| message.is_error),
            last_solve_report: self.document.last_solve_report.as_ref(),
        };

        let mut response = render_shell(
            ui,
            &mut shell,
            interaction_controller::handle_viewport_interaction,
        );
        self.handle_global_shortcuts(ui, &mut response);

        self.dispatch(std::mem::take(&mut response.commands));
        self.handle_window_close_request(ui);
        self.handle_shell_actions(ui, &response);
        self.render_file_dialog(ui);
        self.render_discard_changes_dialog(ui);
        self.sync_viewport_title(ui);
    }
}

fn command_mutates_document(command: &AppCommand) -> bool {
    !matches!(
        command,
        AppCommand::SetActiveSketch(_)
            | AppCommand::SelectSingle { .. }
            | AppCommand::SelectBody(_)
            | AppCommand::SelectBodies { .. }
            | AppCommand::SelectEntities { .. }
            | AppCommand::SelectVertices { .. }
            | AppCommand::ToggleSelection { .. }
            | AppCommand::ClearSelection
            | AppCommand::NoOp
    )
}

fn suggested_project_path(project_name: &str, current_path: Option<&Path>) -> PathBuf {
    if let Some(path) = current_path {
        return path.to_path_buf();
    }

    let mut base = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.push(default_project_filename(project_name));
    base
}

fn document_label(project_name: &str, path: Option<&Path>) -> String {
    path.and_then(|current| current.file_name())
        .map(|file_name| file_name.to_string_lossy().into_owned())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| project_name.to_string())
}

fn normalize_project_save_path(path: PathBuf) -> PathBuf {
    let Some(file_name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
    else {
        return path;
    };
    if file_name.contains('.') {
        return path;
    }

    let mut normalized = path;
    normalized.set_file_name(format!("{file_name}.roncad.json"));
    normalized
}

fn dedupe_recent_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut recent_files = Vec::new();
    for path in paths.into_iter().rev() {
        push_recent_file(&mut recent_files, path);
    }
    recent_files
}

fn push_recent_file(recent_files: &mut Vec<PathBuf>, path: PathBuf) {
    recent_files.retain(|existing| existing != &path);
    recent_files.insert(0, path);
    recent_files.truncate(MAX_RECENT_FILES);
}

fn viewport_title(project_name: &str, path: Option<&Path>, dirty: bool) -> String {
    let mut label = document_label(project_name, path);
    if dirty {
        label.push('*');
    }
    format!("{label} - RONCAD")
}

fn default_project_filename(project_name: &str) -> String {
    let mut slug = project_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '_',
        })
        .collect::<String>();
    slug = slug.trim_matches('_').to_string();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    if slug.is_empty() {
        slug = "untitled".to_string();
    }
    format!("{slug}.roncad.json")
}

fn project_path_input_id() -> Id {
    Id::new("project_path_input")
}

#[cfg(test)]
mod tests {
    use super::{
        command_mutates_document, dedupe_recent_files, default_project_filename, document_label,
        normalize_project_save_path, push_recent_file, viewport_title, MAX_RECENT_FILES,
    };
    use roncad_core::command::AppCommand;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_project_filename_normalizes_name() {
        assert_eq!(
            default_project_filename("My First Project"),
            "my_first_project.roncad.json"
        );
    }

    #[test]
    fn selection_commands_do_not_mark_document_dirty() {
        assert!(!command_mutates_document(&AppCommand::ClearSelection));
        assert!(!command_mutates_document(&AppCommand::NoOp));
    }

    #[test]
    fn document_label_prefers_filename() {
        assert_eq!(
            document_label("Untitled", Some(Path::new("/tmp/bracket.roncad.json"))),
            "bracket.roncad.json"
        );
    }

    #[test]
    fn viewport_title_marks_dirty_documents() {
        assert_eq!(
            viewport_title(
                "Untitled",
                Some(Path::new("/tmp/bracket.roncad.json")),
                true
            ),
            "bracket.roncad.json* - RONCAD"
        );
    }

    #[test]
    fn normalize_project_save_path_appends_default_extension() {
        assert_eq!(
            normalize_project_save_path(PathBuf::from("/tmp/bracket")),
            PathBuf::from("/tmp/bracket.roncad.json")
        );
        assert_eq!(
            normalize_project_save_path(PathBuf::from("/tmp/bracket.json")),
            PathBuf::from("/tmp/bracket.json")
        );
    }

    #[test]
    fn push_recent_file_deduplicates_and_caps_length() {
        let mut recent_files = vec![
            PathBuf::from("/tmp/a.roncad.json"),
            PathBuf::from("/tmp/b.roncad.json"),
            PathBuf::from("/tmp/c.roncad.json"),
        ];

        push_recent_file(&mut recent_files, PathBuf::from("/tmp/b.roncad.json"));
        assert_eq!(recent_files[0], PathBuf::from("/tmp/b.roncad.json"));
        assert_eq!(recent_files.len(), 3);

        for index in 0..(MAX_RECENT_FILES + 2) {
            push_recent_file(
                &mut recent_files,
                PathBuf::from(format!("/tmp/{index}.roncad.json")),
            );
        }
        assert_eq!(recent_files.len(), MAX_RECENT_FILES);
    }

    #[test]
    fn dedupe_recent_files_preserves_most_recent_order() {
        let recent_files = dedupe_recent_files(vec![
            PathBuf::from("/tmp/a.roncad.json"),
            PathBuf::from("/tmp/b.roncad.json"),
            PathBuf::from("/tmp/a.roncad.json"),
        ]);
        assert_eq!(
            recent_files,
            vec![
                PathBuf::from("/tmp/a.roncad.json"),
                PathBuf::from("/tmp/b.roncad.json"),
            ]
        );
    }
}
