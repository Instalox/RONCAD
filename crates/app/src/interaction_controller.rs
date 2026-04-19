//! App-side viewport interaction controller.
//! It interprets egui events for the central viewport, updates tool runtime
//! state, and emits AppCommand instances while the UI crate stays focused on
//! layout and painting.

use egui::{Key, PointerButton, Pos2, Rect, Ui};
use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_geometry::{pick_closed_profile, pick_entity, HoverTarget};
use roncad_tools::{
    ActiveToolKind, Modifiers, SnapEngine, SnapResult, ToolContext, ENTITY_PICK_RADIUS_PX,
};
use roncad_ui::{ShellContext, ShellResponse, ViewportInteractionState};

pub fn handle_viewport_interaction(
    ui: &Ui,
    resp: &egui::Response,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
) -> ViewportInteractionState {
    let center = screen_center(rect);
    if resp.clicked() {
        resp.request_focus();
    }
    let palette_open = shell.command_palette.is_open();
    if !palette_open {
        handle_tool_shortcuts(ui, shell.tool_manager);
    }
    let active_kind = shell.tool_manager.active_kind();

    let raw_cursor_world = resp.hover_pos().and_then(|pointer| {
        active_workplane(shell).and_then(|plane| {
            shell
                .camera
                .screen_to_workplane(pos_to_dvec(pointer), center, plane)
        })
    });

    let modifiers = ui.ctx().input(|input| Modifiers {
        shift: input.modifiers.shift,
        ctrl: input.modifiers.ctrl,
        alt: input.modifiers.alt,
    });

    let ctx = ToolContext {
        active_sketch: shell.project.active_sketch,
        sketch: shell.project.active_sketch(),
        pixels_per_mm: shell.camera.pixels_per_mm,
        modifiers,
    };
    let hovered_target = hovered_target(raw_cursor_world, active_kind, &ctx);
    let snap_result = raw_cursor_world
        .and_then(|world| active_snap_result(world, active_kind, shell.snap_engine, &ctx));
    *shell.snap_result = snap_result;
    let cursor_world = raw_cursor_world.map(|world| snap_result.map_or(world, |snap| snap.point));
    *shell.cursor_world_mm = cursor_world;

    if let Some(world) = cursor_world {
        shell.tool_manager.on_pointer_move(&ctx, world);
    }

    if !palette_open {
        handle_dynamic_input(ui, shell, cursor_world, &ctx, response);
    }

    if active_kind == ActiveToolKind::Extrude && resp.clicked_by(PointerButton::Primary) {
        if let Some(HoverTarget::Profile { sketch, profile }) = hovered_target.as_ref() {
            shell.extrude_hud.arm(*sketch, profile.clone());
        }
    } else if resp.clicked_by(PointerButton::Primary) {
        if let Some(pointer) = resp.interact_pointer_pos() {
            let Some(raw_world) = active_workplane(shell).and_then(|plane| {
                shell
                    .camera
                    .screen_to_workplane(pos_to_dvec(pointer), center, plane)
            }) else {
                return ViewportInteractionState { hovered_target };
            };
            let world = active_snap_result(raw_world, active_kind, shell.snap_engine, &ctx)
                .map_or(raw_world, |snap| snap.point);
            let commands = shell.tool_manager.on_pointer_click(&ctx, world);
            response.commands.extend(commands);
        }
    }

    if active_kind == ActiveToolKind::Extrude && resp.clicked_by(PointerButton::Secondary) {
        shell.extrude_hud.clear();
    } else if resp.clicked_by(PointerButton::Secondary) {
        if let Some(pointer) = resp.interact_pointer_pos() {
            let Some(raw_world) = active_workplane(shell).and_then(|plane| {
                shell
                    .camera
                    .screen_to_workplane(pos_to_dvec(pointer), center, plane)
            }) else {
                return ViewportInteractionState { hovered_target };
            };
            let world = active_snap_result(raw_world, active_kind, shell.snap_engine, &ctx)
                .map_or(raw_world, |snap| snap.point);
            let commands = shell.tool_manager.on_pointer_secondary_click(&ctx, world);
            response.commands.extend(commands);
        }
    }

    if !palette_open
        && ui
            .ctx()
            .input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
    {
        if active_kind == ActiveToolKind::Extrude && shell.extrude_hud.is_open() {
            shell.extrude_hud.clear();
        } else {
            let _ = shell.tool_manager.on_escape();
        }
    }

    if !palette_open && resp.has_focus() && ui.ctx().input(|input| input.key_pressed(Key::Delete)) {
        response.commands.push(AppCommand::DeleteSelection);
    }

    let pointer_delta = ui.ctx().input(|input| input.pointer.delta());
    let pointer_delta = DVec2::new(pointer_delta.x as f64, pointer_delta.y as f64);

    if resp.dragged_by(PointerButton::Middle) {
        shell.camera.orbit_pixels(pointer_delta);
    } else if resp.dragged_by(PointerButton::Secondary) {
        if let Some(workplane) = active_workplane(shell).cloned() {
            shell
                .camera
                .pan_pixels_on_workplane(pointer_delta, center, &workplane);
        } else {
            shell.camera.pan_pixels(pointer_delta, center);
        }
    }

    if resp.hovered() {
        let scroll = ui.ctx().input(|input| input.smooth_scroll_delta.y);
        if scroll.abs() > f32::EPSILON {
            if let Some(pointer) = resp.hover_pos() {
                let factor = (scroll as f64 * 0.0025).exp();
                if let Some(workplane) = active_workplane(shell).cloned() {
                    shell.camera.zoom_about_workplane(
                        pos_to_dvec(pointer),
                        center,
                        factor,
                        &workplane,
                    );
                } else {
                    shell
                        .camera
                        .zoom_about(pos_to_dvec(pointer), center, factor);
                }
            }
        }
    }

    ViewportInteractionState { hovered_target }
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

fn hovered_target(
    raw_world: Option<DVec2>,
    active_kind: ActiveToolKind,
    ctx: &ToolContext<'_>,
) -> Option<HoverTarget> {
    match active_kind {
        ActiveToolKind::Select => {
            let sketch_id = ctx.active_sketch?;
            let sketch = ctx.sketch?;
            let world = raw_world?;
            let tolerance_mm = ENTITY_PICK_RADIUS_PX / ctx.pixels_per_mm.max(f64::EPSILON);
            pick_entity(sketch, world, tolerance_mm)
                .map(|entity| HoverTarget::sketch_entity(sketch_id, entity))
        }
        ActiveToolKind::Extrude => {
            let sketch_id = ctx.active_sketch?;
            let sketch = ctx.sketch?;
            let world = raw_world?;
            pick_closed_profile(sketch, world)
                .map(|profile| HoverTarget::profile(sketch_id, profile))
        }
        _ => None,
    }
}

fn tool_uses_snap(kind: ActiveToolKind) -> bool {
    matches!(
        kind,
        ActiveToolKind::Line
            | ActiveToolKind::Rectangle
            | ActiveToolKind::Circle
            | ActiveToolKind::Arc
            | ActiveToolKind::Dimension
    )
}

fn handle_dynamic_input(
    ui: &Ui,
    shell: &mut ShellContext<'_>,
    cursor_world: Option<DVec2>,
    ctx: &ToolContext<'_>,
    response: &mut ShellResponse,
) {
    if !shell.tool_manager.prepare_dynamic_input() {
        return;
    }

    // If a TextEdit (e.g., the selection mini HUD) owns the keyboard, defer.
    // Note: egui_wants_keyboard_input is true for any focused widget, including
    // the viewport response itself, so we need text_edit_focused specifically.
    if ui.ctx().text_edit_focused() {
        return;
    }

    let shift = egui::Modifiers::SHIFT;
    let typed_chars: Vec<char> = ui.ctx().input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .flat_map(str::chars)
            .collect()
    });
    let none = egui::Modifiers::NONE;
    let (backspace, cycle_back, cycle_next, submit) = ui.ctx().input_mut(|input| {
        (
            input.consume_key(none, Key::Backspace),
            input.consume_key(shift, Key::Tab),
            input.consume_key(none, Key::Tab),
            input.consume_key(none, Key::Enter),
        )
    });

    shell.tool_manager.append_dynamic_chars(typed_chars);
    if backspace {
        let _ = shell.tool_manager.backspace_dynamic_input();
    }

    if cycle_back {
        shell.tool_manager.cycle_dynamic_input_back();
    } else if cycle_next {
        shell.tool_manager.cycle_dynamic_input();
    }
    if submit {
        if let Some(world) = cursor_world {
            let commands = shell.tool_manager.commit_dynamic(ctx, world);
            response.commands.extend(commands);
        }
    }
}

fn handle_tool_shortcuts(ui: &Ui, manager: &mut roncad_tools::ToolManager) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    if !manager.dynamic_fields().is_empty() {
        return;
    }

    let next = ui.ctx().input(|input| {
        if input.modifiers.ctrl || input.modifiers.alt || input.modifiers.command {
            return None;
        }
        if input.key_pressed(Key::V) {
            Some(ActiveToolKind::Select)
        } else if input.key_pressed(Key::H) {
            Some(ActiveToolKind::Pan)
        } else if input.key_pressed(Key::A) {
            Some(ActiveToolKind::Arc)
        } else if input.key_pressed(Key::L) {
            Some(ActiveToolKind::Line)
        } else if input.key_pressed(Key::R) {
            Some(ActiveToolKind::Rectangle)
        } else if input.key_pressed(Key::C) {
            Some(ActiveToolKind::Circle)
        } else if input.key_pressed(Key::F) {
            Some(ActiveToolKind::Fillet)
        } else if input.key_pressed(Key::D) {
            Some(ActiveToolKind::Dimension)
        } else if input.key_pressed(Key::E) {
            Some(ActiveToolKind::Extrude)
        } else {
            None
        }
    });

    if let Some(kind) = next {
        manager.set_active(kind);
    }
}

fn screen_center(rect: Rect) -> DVec2 {
    DVec2::new(
        (rect.min.x as f64 + rect.max.x as f64) * 0.5,
        (rect.min.y as f64 + rect.max.y as f64) * 0.5,
    )
}

fn pos_to_dvec(pos: Pos2) -> DVec2 {
    DVec2::new(pos.x as f64, pos.y as f64)
}

fn active_workplane<'a>(shell: &'a ShellContext<'_>) -> Option<&'a roncad_geometry::Workplane> {
    shell.project.active_workplane()
}
