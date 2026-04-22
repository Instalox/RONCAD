use std::path::PathBuf;

use egui::{
    vec2, Align, Align2, Color32, Context, FontId, Frame, Id, Label, Layout, Margin, Modal,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Vec2,
};
use egui_phosphor::regular as ph;
use roncad_core::command::AppCommand;
use roncad_tools::ActiveToolKind;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const PALETTE_ROW_HEIGHT: f32 = 28.0;
const PALETTE_MAX_HEIGHT: f32 = 320.0;

const TOOLS: &[ActiveToolKind] = &[
    ActiveToolKind::Select,
    ActiveToolKind::Pan,
    ActiveToolKind::Line,
    ActiveToolKind::Rectangle,
    ActiveToolKind::Circle,
    ActiveToolKind::Arc,
    ActiveToolKind::Fillet,
    ActiveToolKind::Extrude,
    ActiveToolKind::Revolve,
];

#[derive(Debug, Default)]
pub struct CommandPaletteState {
    open: bool,
    query: String,
    selected: usize,
    request_focus: bool,
}

impl CommandPaletteState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.request_focus = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.request_focus = false;
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    fn clamp_selected(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(len - 1);
        }
    }
}

#[derive(Clone)]
enum PaletteAction {
    Tool(ActiveToolKind),
    Command(AppCommand),
    FitView,
    OpenProject,
    OpenProjectByPath,
    OpenProjectPath(PathBuf),
    SaveProject,
    SaveProjectAs,
    SaveProjectAsByPath,
}

#[derive(Clone)]
struct PaletteItem {
    group: &'static str,
    icon: &'static str,
    label: String,
    detail: Option<String>,
    shortcut: Option<&'static str>,
    search_text: String,
    action: PaletteAction,
}

pub fn handle_shortcut(ctx: &Context, state: &mut CommandPaletteState) {
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
        state.toggle();
    }
}

pub fn render(ctx: &Context, shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
    if !shell.command_palette.is_open() {
        return;
    }

    let modal_id = Id::new("command_palette_modal");
    let catalog = build_catalog(shell);
    let modal = Modal::new(modal_id)
        .area(Modal::default_area(modal_id).anchor(Align2::CENTER_TOP, vec2(0.0, 54.0)))
        .backdrop_color(Color32::from_black_alpha(24))
        .frame(
            Frame::new()
                .fill(ThemeColors::BG_PANEL)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(10, 8))
                .corner_radius(4.0_f32),
        );

    let modal_response = modal.show(ctx, |ui| {
        ui.set_width(palette_width(ctx));

        if shell.command_palette.request_focus {
            ctx.memory_mut(|memory| memory.request_focus(input_id()));
            shell.command_palette.request_focus = false;
        }

        let query_changed = render_query_row(ui, shell.command_palette);
        ui.add_space(8.0);

        let items = filter_catalog(&catalog, &shell.command_palette.query);
        if query_changed {
            shell.command_palette.selected = 0;
        }
        shell.command_palette.clamp_selected(items.len());
        if let Some(action) = handle_list_navigation(ctx, shell.command_palette, &items) {
            return Some(action);
        }

        render_item_groups(ui, shell.command_palette, &items)
    });

    if let Some(action) = modal_response.inner {
        run_action(action, shell, response);
        shell.command_palette.close();
    } else if modal_response.should_close() {
        shell.command_palette.close();
    }
}

fn build_catalog(shell: &ShellContext<'_>) -> Vec<PaletteItem> {
    let mut items = Vec::new();
    let active_tool = shell.tool_manager.active_kind();
    let file_detail = shell
        .document_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "No file path yet".to_string());

    items.push(PaletteItem {
        group: "Project",
        icon: ph::FOLDER_OPEN,
        label: "Open project".to_string(),
        detail: Some("Native file picker".to_string()),
        shortcut: Some("Mod+O"),
        search_text: "open load project file json".to_string(),
        action: PaletteAction::OpenProject,
    });
    items.push(PaletteItem {
        group: "Project",
        icon: ph::TEXTBOX,
        label: "Open project by path".to_string(),
        detail: Some(file_detail.clone()),
        shortcut: None,
        search_text: "open load project file path json manual".to_string(),
        action: PaletteAction::OpenProjectByPath,
    });
    items.push(PaletteItem {
        group: "Project",
        icon: ph::FLOPPY_DISK,
        label: "Save project".to_string(),
        detail: Some(if shell.document_dirty {
            "Unsaved changes".to_string()
        } else {
            "Up to date".to_string()
        }),
        shortcut: Some("Mod+S"),
        search_text: "save write project file json".to_string(),
        action: PaletteAction::SaveProject,
    });
    items.push(PaletteItem {
        group: "Project",
        icon: ph::FLOPPY_DISK_BACK,
        label: "Save project as".to_string(),
        detail: Some("Native file picker".to_string()),
        shortcut: Some("Mod+Shift+S"),
        search_text: "save as write project file path json".to_string(),
        action: PaletteAction::SaveProjectAs,
    });
    items.push(PaletteItem {
        group: "Project",
        icon: ph::TEXTBOX,
        label: "Save project as path".to_string(),
        detail: Some(file_detail),
        shortcut: None,
        search_text: "save as write project file path json manual".to_string(),
        action: PaletteAction::SaveProjectAsByPath,
    });

    for path in shell.recent_files {
        if shell
            .document_path
            .is_some_and(|current| current == path.as_path())
        {
            continue;
        }

        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| path.display().to_string());
        items.push(PaletteItem {
            group: "Recent",
            icon: ph::FOLDER_OPEN,
            label: format!("Open recent {}", file_name),
            detail: Some(path.display().to_string()),
            shortcut: None,
            search_text: format!("recent open project {} {}", file_name, path.display()),
            action: PaletteAction::OpenProjectPath(path.clone()),
        });
    }

    for tool in TOOLS {
        let detail = if *tool == active_tool {
            Some("Active".to_string())
        } else {
            tool_palette_detail(*tool).map(str::to_string)
        };
        items.push(PaletteItem {
            group: "Tools",
            icon: tool_glyph(*tool),
            label: tool.label().to_string(),
            detail,
            shortcut: tool.shortcut(),
            search_text: format!(
                "{} {} {}",
                tool.label(),
                tool.hint(),
                tool_palette_detail(*tool).unwrap_or_default()
            ),
            action: PaletteAction::Tool(*tool),
        });
    }

    for (plane_id, plane) in shell.project.workplanes.iter() {
        items.push(PaletteItem {
            group: "Project",
            icon: ph::PLUS,
            label: format!("Create sketch on {}", plane.name),
            detail: Some(format!(
                "Sketch {}",
                shell.project.sketches.len().saturating_add(1)
            )),
            shortcut: None,
            search_text: format!("new sketch create project {} plane", plane.name),
            action: PaletteAction::Command(AppCommand::CreateSketch {
                name: format!("Sketch {}", shell.project.sketches.len() + 1),
                plane: plane_id,
            }),
        });
    }

    if let Some(sketch) = shell.project.active_sketch() {
        items.push(PaletteItem {
            group: "View",
            icon: ph::PROJECTOR_SCREEN,
            label: "Fit active sketch".to_string(),
            detail: Some(format!("{} entities", sketch.entities.len())),
            shortcut: None,
            search_text: "fit frame zoom active sketch view".to_string(),
            action: PaletteAction::FitView,
        });
    }

    if !shell.selection.is_empty() {
        let count = shell.selection.len();
        let selection_label = if count == 1 { "item" } else { "items" };
        items.push(PaletteItem {
            group: "Selection",
            icon: ph::TRASH,
            label: "Delete selection".to_string(),
            detail: Some(format!("{count} {selection_label}")),
            shortcut: Some("Del"),
            search_text: "delete remove selection".to_string(),
            action: PaletteAction::Command(AppCommand::DeleteSelection),
        });
        items.push(PaletteItem {
            group: "Selection",
            icon: ph::X,
            label: "Clear selection".to_string(),
            detail: Some(format!("{count} {selection_label}")),
            shortcut: None,
            search_text: "clear deselect selection".to_string(),
            action: PaletteAction::Command(AppCommand::ClearSelection),
        });
    }

    for (id, sketch) in shell.project.sketches.iter() {
        let current = shell.project.active_sketch == Some(id);
        let detail = if current {
            format!("Current · {} entities", sketch.entities.len())
        } else {
            format!("{} entities", sketch.entities.len())
        };
        items.push(PaletteItem {
            group: "Sketches",
            icon: ph::SQUARE,
            label: format!("Activate {}", sketch.name),
            detail: Some(detail),
            shortcut: None,
            search_text: format!("activate sketch {} current", sketch.name),
            action: PaletteAction::Command(AppCommand::SetActiveSketch(id)),
        });
    }

    items
}

fn render_query_row(ui: &mut Ui, state: &mut CommandPaletteState) -> bool {
    Frame::new()
        .fill(ThemeColors::BG_DEEP)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(8, 6))
        .corner_radius(3.0_f32)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    ThemeColors::TEXT_MID,
                    RichText::new(ph::MAGNIFYING_GLASS).size(12.0),
                );
                let input_width = (ui.available_width() - 34.0).max(72.0);
                let response = ui.add_sized(
                    [input_width, 18.0],
                    TextEdit::singleline(&mut state.query)
                        .id(input_id())
                        .frame(Frame::NONE)
                        .hint_text("Search tools, commands, sketches…"),
                );
                keycap(ui, "Esc");
                response
            })
            .inner
        })
        .inner
        .changed()
}

fn handle_list_navigation(
    ctx: &Context,
    state: &mut CommandPaletteState,
    items: &[&PaletteItem],
) -> Option<PaletteAction> {
    let none = egui::Modifiers::NONE;
    let (down, up, submit) = ctx.input_mut(|input| {
        (
            input.consume_key(none, egui::Key::ArrowDown),
            input.consume_key(none, egui::Key::ArrowUp),
            input.consume_key(none, egui::Key::Enter),
        )
    });

    if down && !items.is_empty() {
        state.selected = (state.selected + 1).min(items.len() - 1);
    }
    if up && !items.is_empty() {
        state.selected = state.selected.saturating_sub(1);
    }
    if submit && !items.is_empty() {
        return Some(items[state.selected].action.clone());
    }

    None
}

fn render_item_groups(
    ui: &mut Ui,
    state: &mut CommandPaletteState,
    items: &[&PaletteItem],
) -> Option<PaletteAction> {
    if items.is_empty() {
        let height = 96.0;
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(rect, 3.0, ThemeColors::BG_DEEP);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No matching commands",
            FontId::proportional(12.0),
            ThemeColors::TEXT_DIM,
        );
        return None;
    }

    let mut action = None;
    ScrollArea::vertical()
        .max_height(PALETTE_MAX_HEIGHT)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut current_group = None;
            for (index, item) in items.iter().enumerate() {
                if current_group != Some(item.group) {
                    current_group = Some(item.group);
                    if index > 0 {
                        ui.add_space(6.0);
                    }
                    ui.colored_label(
                        ThemeColors::TEXT_DIM,
                        RichText::new(item.group).monospace().size(10.0),
                    );
                    ui.add_space(2.0);
                }

                let row = render_item_row(ui, item, index == state.selected);
                if row.hovered {
                    state.selected = index;
                }
                if let Some(clicked) = row.action {
                    action = Some(clicked);
                }
            }
        });

    action
}

struct RowResult {
    hovered: bool,
    action: Option<PaletteAction>,
}

fn render_item_row(ui: &mut Ui, item: &PaletteItem, selected: bool) -> RowResult {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), PALETTE_ROW_HEIGHT),
        Sense::click(),
    );
    let fill = if selected {
        ThemeColors::BG_HEADER_ACTIVE
    } else if response.hovered() {
        ThemeColors::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 2.0, fill);
    }
    if selected {
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0, ThemeColors::ACCENT_DIM),
            StrokeKind::Outside,
        );
    }

    let text_color = if selected {
        ThemeColors::TEXT
    } else {
        ThemeColors::TEXT_MID
    };
    let mut row_ui = ui.new_child(
        UiBuilder::new()
            .id_salt(("palette_row", &item.label))
            .max_rect(rect.shrink2(vec2(8.0, 4.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    row_ui.colored_label(
        if selected {
            ThemeColors::ACCENT
        } else {
            ThemeColors::TEXT_DIM
        },
        RichText::new(item.icon).size(12.0),
    );
    row_ui.add_space(6.0);
    row_ui.colored_label(text_color, RichText::new(&item.label).size(12.0));
    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if let Some(shortcut) = item.shortcut {
            keycap(ui, shortcut);
        }
        if let Some(detail) = &item.detail {
            ui.add(
                Label::new(
                    RichText::new(detail)
                        .size(10.5)
                        .color(ThemeColors::TEXT_DIM),
                )
                .truncate(),
            );
        }
    });

    RowResult {
        hovered: response.hovered(),
        action: response.clicked().then(|| item.action.clone()),
    }
}

fn filter_catalog<'a>(catalog: &'a [PaletteItem], query: &str) -> Vec<&'a PaletteItem> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return catalog.iter().collect();
    }

    catalog
        .iter()
        .filter(|item| {
            item.label.to_ascii_lowercase().contains(&query)
                || item.group.to_ascii_lowercase().contains(&query)
                || item.search_text.to_ascii_lowercase().contains(&query)
                || item
                    .detail
                    .as_deref()
                    .map(|detail| detail.to_ascii_lowercase().contains(&query))
                    .unwrap_or(false)
        })
        .collect()
}

fn run_action(action: PaletteAction, shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
    match action {
        PaletteAction::Tool(kind) => shell.tool_manager.set_active(kind),
        PaletteAction::Command(command) => response.commands.push(command),
        PaletteAction::FitView => response.fit_view_requested = true,
        PaletteAction::OpenProject => response.open_project_requested = true,
        PaletteAction::OpenProjectByPath => response.open_project_by_path_requested = true,
        PaletteAction::OpenProjectPath(path) => response.open_project_path = Some(path),
        PaletteAction::SaveProject => response.save_project_requested = true,
        PaletteAction::SaveProjectAs => response.save_project_as_requested = true,
        PaletteAction::SaveProjectAsByPath => response.save_project_as_path_requested = true,
    }
}

fn keycap(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(ThemeColors::BG_DEEP)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(4, 1))
        .corner_radius(2.0_f32)
        .show(ui, |ui| {
            ui.colored_label(
                ThemeColors::TEXT_MID,
                RichText::new(text).monospace().size(9.0),
            );
        });
}

fn palette_width(ctx: &Context) -> f32 {
    let available = (ctx.content_rect().width() - 32.0).max(0.0);
    if available < 320.0 {
        available
    } else {
        available.min(560.0)
    }
}

fn input_id() -> Id {
    Id::new("command_palette_input")
}

fn tool_glyph(tool: ActiveToolKind) -> &'static str {
    match tool {
        ActiveToolKind::Select => ph::CURSOR,
        ActiveToolKind::Pan => ph::HAND,
        ActiveToolKind::Line => ph::LINE_SEGMENT,
        ActiveToolKind::Rectangle => ph::RECTANGLE,
        ActiveToolKind::Circle => ph::CIRCLE,
        ActiveToolKind::Arc => "A",
        ActiveToolKind::Fillet => "F",
        ActiveToolKind::Dimension => ph::RULER,
        ActiveToolKind::Extrude => ph::ARROW_FAT_LINE_UP,
        ActiveToolKind::Revolve => ph::ARROWS_CLOCKWISE,
    }
}

fn tool_palette_detail(tool: ActiveToolKind) -> Option<&'static str> {
    match tool {
        ActiveToolKind::Select => Some("Pick and edit geometry"),
        ActiveToolKind::Pan => Some("Drag the view"),
        ActiveToolKind::Line => Some("Draw chained segments"),
        ActiveToolKind::Rectangle => Some("Draw corner to corner"),
        ActiveToolKind::Circle => Some("Place center and radius"),
        ActiveToolKind::Arc => Some("Place center, start, end"),
        ActiveToolKind::Fillet => Some("Round a sketch corner"),
        ActiveToolKind::Dimension => Some("Place a distance dimension"),
        ActiveToolKind::Extrude => Some("Preview closed profiles"),
        ActiveToolKind::Revolve => Some("Revolve profile around axis"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item() -> PaletteItem {
        PaletteItem {
            group: "Tools",
            icon: ph::LINE_SEGMENT,
            label: "Line".to_string(),
            detail: Some("Draw chained segments".to_string()),
            shortcut: Some("L"),
            search_text: "line draw chained segments".to_string(),
            action: PaletteAction::Tool(ActiveToolKind::Line),
        }
    }

    #[test]
    fn filter_matches_label_detail_and_group() {
        let item = sample_item();
        let catalog = vec![item];

        assert_eq!(filter_catalog(&catalog, "line").len(), 1);
        assert_eq!(filter_catalog(&catalog, "chained").len(), 1);
        assert_eq!(filter_catalog(&catalog, "tools").len(), 1);
        assert!(filter_catalog(&catalog, "extrude").is_empty());
    }

    #[test]
    fn selection_clamps_when_results_shrink() {
        let mut state = CommandPaletteState {
            open: true,
            query: String::new(),
            selected: 8,
            request_focus: false,
        };

        state.clamp_selected(2);
        assert_eq!(state.selected, 1);

        state.clamp_selected(0);
        assert_eq!(state.selected, 0);
    }
}
