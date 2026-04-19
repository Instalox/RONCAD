//! Inline "press-and-type" mini HUD. When a single sketch entity is selected,
//! a compact panel floats near the entity exposing the values that define it.
//! Typing a number and pressing Enter submits a SetXxx command without the
//! user opening the inspector. The HUD also echoes the active snap and tool
//! so context stays close to the cursor.

use egui::{Area, Frame, Id, Key, Margin, Order, Pos2, Rect, Stroke, Ui};
use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_core::selection::{Selection, SelectionItem};
use roncad_core::units::LengthMm;
use roncad_geometry::{arc_mid_point, SketchEntity};

use super::{screen_center, to_pos};
use crate::hud_state::HudEditState;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub(super) fn paint(ui: &mut Ui, rect: Rect, shell: &mut ShellContext<'_>, response: &mut ShellResponse) {
    let Some((sketch_id, entity_id)) = single_selected_entity(shell.selection) else {
        shell.hud_state.clear();
        return;
    };

    let Some(entity) = shell
        .project
        .sketches
        .get(sketch_id)
        .and_then(|s| s.entities.get(entity_id))
        .cloned()
    else {
        shell.hud_state.clear();
        return;
    };

    let fields = fields_for(&entity);
    ensure_buffers(shell.hud_state, sketch_id, entity_id, &fields);

    let anchor_world = anchor_world(&entity);
    let center = screen_center(rect);
    let screen = to_pos(shell.camera.world_to_screen(anchor_world, center));
    let hud_pos = Pos2::new(screen.x + 14.0, screen.y + 14.0);

    let snap_label = shell
        .snap_result
        .as_ref()
        .and_then(|snap| snap.kind.map(|kind| kind.label()));
    let tool_kind = shell.tool_manager.active_kind();

    Area::new(Id::new("mini_hud_selection"))
        .order(Order::Foreground)
        .fixed_pos(hud_pos)
        .constrain_to(rect)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(8, 6))
                .corner_radius(3.0_f32)
                .show(ui, |ui| {
                    ui.set_max_width(220.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(ThemeColors::ACCENT, entity.kind_name());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(label) = snap_label {
                                        ui.colored_label(ThemeColors::ACCENT_AMBER, label);
                                    }
                                },
                            );
                        });
                        ui.add_space(2.0);

                        for (index, field) in fields.iter().enumerate() {
                            render_field(
                                ui,
                                shell.hud_state,
                                index,
                                field,
                                sketch_id,
                                entity_id,
                                response,
                            );
                        }

                        ui.add_space(2.0);
                        ui.colored_label(
                            ThemeColors::TEXT_DIM,
                            format!("{} · Enter commits · Esc clears", tool_kind.label()),
                        );
                    });
                });
        });
}

fn single_selected_entity(selection: &Selection) -> Option<(SketchId, SketchEntityId)> {
    if selection.len() != 1 {
        return None;
    }
    selection.iter().find_map(|item| match item {
        SelectionItem::SketchEntity { sketch, entity } => Some((*sketch, *entity)),
        _ => None,
    })
}

#[derive(Debug, Clone, Copy)]
enum FieldKind {
    LineLength,
    RectWidth,
    RectHeight,
    CircleRadius,
    CircleDiameter,
    ArcRadius,
    ArcSweepDeg,
    PointX,
    PointY,
}

struct Field {
    label: &'static str,
    unit: &'static str,
    value: f64,
    kind: FieldKind,
}

fn fields_for(entity: &SketchEntity) -> Vec<Field> {
    match entity {
        SketchEntity::Point { p } => vec![
            Field {
                label: "X",
                unit: "mm",
                value: p.x,
                kind: FieldKind::PointX,
            },
            Field {
                label: "Y",
                unit: "mm",
                value: p.y,
                kind: FieldKind::PointY,
            },
        ],
        SketchEntity::Line { a, b } => vec![Field {
            label: "Length",
            unit: "mm",
            value: a.distance(*b),
            kind: FieldKind::LineLength,
        }],
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let delta = *corner_b - *corner_a;
            vec![
                Field {
                    label: "Width",
                    unit: "mm",
                    value: delta.x.abs(),
                    kind: FieldKind::RectWidth,
                },
                Field {
                    label: "Height",
                    unit: "mm",
                    value: delta.y.abs(),
                    kind: FieldKind::RectHeight,
                },
            ]
        }
        SketchEntity::Circle { radius, .. } => vec![
            Field {
                label: "Radius",
                unit: "mm",
                value: *radius,
                kind: FieldKind::CircleRadius,
            },
            Field {
                label: "Diameter",
                unit: "mm",
                value: *radius * 2.0,
                kind: FieldKind::CircleDiameter,
            },
        ],
        SketchEntity::Arc {
            radius,
            sweep_angle,
            ..
        } => vec![
            Field {
                label: "Radius",
                unit: "mm",
                value: *radius,
                kind: FieldKind::ArcRadius,
            },
            Field {
                label: "Sweep",
                unit: "deg",
                value: sweep_angle.to_degrees(),
                kind: FieldKind::ArcSweepDeg,
            },
        ],
    }
}

fn ensure_buffers(
    state: &mut HudEditState,
    sketch: SketchId,
    entity: SketchEntityId,
    fields: &[Field],
) {
    let same_entity = state.tracked == Some((sketch, entity));
    let same_shape = state.fields.len() == fields.len();
    if !same_entity || !same_shape {
        state.tracked = Some((sketch, entity));
        state.fields = fields.iter().map(|f| format_value(f.value)).collect();
        state.focus_index = None;
    }
}

fn render_field(
    ui: &mut Ui,
    state: &mut HudEditState,
    index: usize,
    field: &Field,
    sketch: SketchId,
    entity: SketchEntityId,
    response: &mut ShellResponse,
) {
    let focused = state.focus_index == Some(index);
    if !focused {
        state.fields[index] = format_value(field.value);
    }

    let buffer = &mut state.fields[index];
    let mut committed = false;

    ui.horizontal(|ui| {
        ui.colored_label(ThemeColors::TEXT_DIM, field.label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(ThemeColors::TEXT_DIM, field.unit);
            let text_edit = ui.add(
                egui::TextEdit::singleline(buffer)
                    .desired_width(80.0)
                    .font(egui::FontId::monospace(12.0))
                    .background_color(ThemeColors::BG_PANEL_ALT)
                    .text_color(ThemeColors::TEXT),
            );

            if text_edit.gained_focus() {
                state.focus_index = Some(index);
            }

            let submitted = text_edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if submitted {
                committed = true;
            } else if text_edit.lost_focus() && state.focus_index == Some(index) {
                state.focus_index = None;
            }
        });
    });

    if committed {
        if let Ok(value) = state.fields[index].trim().parse::<f64>() {
            if let Some(cmd) = command_for(field.kind, sketch, entity, value) {
                response.commands.push(cmd);
            }
        }
        state.focus_index = None;
    }
}

fn command_for(
    kind: FieldKind,
    sketch: SketchId,
    entity: SketchEntityId,
    value: f64,
) -> Option<AppCommand> {
    match kind {
        FieldKind::LineLength => (value > 0.0).then_some(AppCommand::SetLineLength {
            sketch,
            entity,
            length: LengthMm::new(value),
        }),
        FieldKind::RectWidth => (value > 0.0).then_some(AppCommand::SetRectangleWidth {
            sketch,
            entity,
            width: LengthMm::new(value),
        }),
        FieldKind::RectHeight => (value > 0.0).then_some(AppCommand::SetRectangleHeight {
            sketch,
            entity,
            height: LengthMm::new(value),
        }),
        FieldKind::CircleRadius => (value > 0.0).then_some(AppCommand::SetCircleRadius {
            sketch,
            entity,
            radius: LengthMm::new(value),
        }),
        FieldKind::CircleDiameter => (value > 0.0).then_some(AppCommand::SetCircleRadius {
            sketch,
            entity,
            radius: LengthMm::new(value * 0.5),
        }),
        FieldKind::ArcRadius => (value > 0.0).then_some(AppCommand::SetArcRadius {
            sketch,
            entity,
            radius: LengthMm::new(value),
        }),
        FieldKind::ArcSweepDeg => Some(AppCommand::SetArcSweepDegrees {
            sketch,
            entity,
            sweep_degrees: value,
        }),
        FieldKind::PointX => Some(AppCommand::SetPointX {
            sketch,
            entity,
            x: value,
        }),
        FieldKind::PointY => Some(AppCommand::SetPointY {
            sketch,
            entity,
            y: value,
        }),
    }
}

fn anchor_world(entity: &SketchEntity) -> DVec2 {
    match entity {
        SketchEntity::Point { p } => *p,
        SketchEntity::Line { a, b } => (*a + *b) * 0.5,
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let max_x = corner_a.x.max(corner_b.x);
            let min_y = corner_a.y.min(corner_b.y);
            DVec2::new(max_x, min_y)
        }
        SketchEntity::Circle { center, radius } => DVec2::new(center.x + *radius, center.y),
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => arc_mid_point(*center, *radius, *start_angle, *sweep_angle),
    }
}

fn format_value(value: f64) -> String {
    format!("{value:.3}")
}
