//! Central viewport. Milestone 2 paints the grid, the sketch entities of the
//! active sketch, and the live preview from the active tool. Pointer events
//! on left-click are routed to the ToolManager; middle/right drag pans.

mod dimension_overlay;
mod grid_overlay;
mod hud_overlay;
mod sketch_overlay;
mod snap_overlay;
mod tool_overlay;

use egui::{CentralPanel, Color32, Frame, Key, PointerButton, Pos2, Rect, Sense, Ui};
use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_geometry::pick_entity;
use roncad_tools::{
    ActiveToolKind, Modifiers, SnapEngine, SnapResult, ToolContext, ENTITY_PICK_RADIUS_PX,
};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const COLOR_SKETCH: Color32 = Color32::from_rgb(0xE0, 0xE4, 0xEA);
const COLOR_PREVIEW: Color32 = Color32::from_rgb(0x4F, 0xA3, 0xF7);

pub fn render(ui: &mut Ui, shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
    CentralPanel::default()
        .frame(Frame::new().fill(ThemeColors::BG_DEEP))
        .show_inside(ui, |ui| {
            let available = ui.available_size_before_wrap();
            let (rect, resp) =
                ui.allocate_exact_size(available, Sense::click_and_drag());

            let hovered_entity = handle_input(ui, &resp, shell, rect, response);
            grid_overlay::paint(ui.painter(), rect, shell.camera);
            sketch_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
                hovered_entity,
            );
            dimension_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
            );
            snap_overlay::paint(ui.painter(), rect, shell.camera, shell.snap_result.as_ref());
            tool_overlay::paint_preview(ui.painter(), rect, shell.camera, shell.tool_manager);
            hud_overlay::paint(ui, rect, shell);
        });
}

fn handle_input(
    ui: &Ui,
    resp: &egui::Response,
    shell: &mut ShellContext<'_>,
    rect: Rect,
    response: &mut ShellResponse,
) -> Option<(SketchId, SketchEntityId)> {
    let center = screen_center(rect);
    if resp.clicked() {
        resp.request_focus();
    }
    let active_kind = shell.tool_manager.active_kind();

    let raw_cursor_world = resp
        .hover_pos()
        .map(|p| shell.camera.screen_to_world(pos_to_dvec(p), center));

    let modifiers = ui.ctx().input(|i| Modifiers {
        shift: i.modifiers.shift,
        ctrl: i.modifiers.ctrl,
        alt: i.modifiers.alt,
    });

    let ctx = ToolContext {
        active_sketch: shell.project.active_sketch,
        sketch: shell.project.active_sketch(),
        pixels_per_mm: shell.camera.pixels_per_mm,
        modifiers,
    };
    let hovered_entity = hovered_selectable_entity(raw_cursor_world, active_kind, &ctx);
    let snap_result = raw_cursor_world.and_then(|world| {
        active_snap_result(world, active_kind, shell.snap_engine, &ctx)
    });
    *shell.snap_result = snap_result;
    let cursor_world = raw_cursor_world.map(|world| {
        snap_result.map_or(world, |snap| snap.point)
    });
    *shell.cursor_world_mm = cursor_world;

    if let Some(world) = cursor_world {
        shell.tool_manager.on_pointer_move(&ctx, world);
    }

    if resp.clicked_by(PointerButton::Primary) {
        if let Some(p) = resp.interact_pointer_pos() {
            let raw_world = shell.camera.screen_to_world(pos_to_dvec(p), center);
            let world = active_snap_result(raw_world, active_kind, shell.snap_engine, &ctx)
                .map_or(raw_world, |snap| snap.point);
            let cmds = shell.tool_manager.on_pointer_click(&ctx, world);
            response.commands.extend(cmds);
        }
    }

    if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
        shell.tool_manager.on_escape();
    }

    if resp.has_focus() && ui.ctx().input(|i| i.key_pressed(Key::Delete)) {
        response.commands.push(AppCommand::DeleteSelection);
    }

    if resp.dragged_by(PointerButton::Middle)
        || resp.dragged_by(PointerButton::Secondary)
    {
        let delta = resp.drag_delta();
        shell
            .camera
            .pan_pixels(DVec2::new(delta.x as f64, delta.y as f64));
    }

    if resp.hovered() {
        let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > f32::EPSILON {
            if let Some(ptr) = resp.hover_pos() {
                let factor = (scroll as f64 * 0.0025).exp();
                shell
                    .camera
                    .zoom_about(pos_to_dvec(ptr), center, factor);
            }
        }
    }

    hovered_entity
}

fn pick_step(pixels_per_mm: f64, min_px: f64) -> f64 {
    let target_mm = min_px / pixels_per_mm.max(f64::EPSILON);
    let pow = target_mm.log10().ceil();
    let base = 10f64.powf(pow);
    for candidate in [base * 0.1, base * 0.2, base * 0.5, base] {
        if candidate * pixels_per_mm >= min_px {
            return candidate;
        }
    }
    base
}

fn active_snap_result(
    raw_world: DVec2,
    active_kind: ActiveToolKind,
    snap_engine: &SnapEngine,
    ctx: &ToolContext<'_>,
) -> Option<SnapResult> {
    if tool_uses_snap(active_kind) {
        let result = snap_engine.snap(raw_world, ctx.sketch, ctx.pixels_per_mm);
        result.kind.map(|_| result)
    } else {
        None
    }
}

fn hovered_selectable_entity(
    raw_world: Option<DVec2>,
    active_kind: ActiveToolKind,
    ctx: &ToolContext<'_>,
) -> Option<(SketchId, SketchEntityId)> {
    if active_kind != ActiveToolKind::Select {
        return None;
    }

    let sketch_id = ctx.active_sketch?;
    let sketch = ctx.sketch?;
    let world = raw_world?;
    let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
    pick_entity(sketch, world, tolerance_mm).map(|entity| (sketch_id, entity))
}

fn tool_uses_snap(kind: ActiveToolKind) -> bool {
    matches!(
        kind,
        ActiveToolKind::Line | ActiveToolKind::Rectangle | ActiveToolKind::Circle
    )
}

fn screen_center(rect: Rect) -> DVec2 {
    DVec2::new(
        (rect.min.x as f64 + rect.max.x as f64) * 0.5,
        (rect.min.y as f64 + rect.max.y as f64) * 0.5,
    )
}

fn pos_to_dvec(p: Pos2) -> DVec2 {
    DVec2::new(p.x as f64, p.y as f64)
}

fn to_pos(v: DVec2) -> Pos2 {
    Pos2::new(v.x as f32, v.y as f32)
}
