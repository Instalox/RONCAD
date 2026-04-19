//! Inline "press-and-type" mini HUD. When a single sketch entity is selected,
//! a compact panel floats near the entity exposing the values that define it.
//! Typing a number and pressing Enter submits a SetXxx command without the
//! user opening the inspector. The HUD also echoes the active snap and tool
//! so context stays close to the cursor.

use egui::{
    text::{CCursor, CCursorRange},
    Area, Frame, Id, Key, Margin, Order, Pos2, Rect, Stroke, Ui, Vec2,
};
use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_core::units::LengthMm;
use roncad_geometry::{arc_mid_point, SketchEntity};

use super::{screen_center, to_pos};
use crate::hud_state::HudEditState;
use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const MINI_HUD_WIDTH: f32 = 176.0;
const MINI_HUD_GAP: f32 = 18.0;
const MINI_HUD_PAD: f32 = 8.0;
const MINI_HUD_BASE_HEIGHT: f32 = 42.0;
const MINI_HUD_ROW_HEIGHT: f32 = 24.0;

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
) {
    let Some(selected) = single_selected_entity(shell.selection) else {
        shell.hud_state.clear();
        return;
    };
    let Some((sketch_id, entity_id)) = sketch_entity_ids(&selected) else {
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
    ensure_buffers(shell.hud_state, &selected, &fields);

    let anchor_world = anchor_world(&entity);
    let center = screen_center(rect);
    let anchor_screen = to_pos(shell.camera.world_to_screen(anchor_world, center));
    let cursor_screen = shell
        .cursor_world_mm
        .as_ref()
        .map(|cursor| to_pos(shell.camera.world_to_screen(*cursor, center)));
    let hud_pos = mini_hud_pos(anchor_screen, cursor_screen, rect, fields.len());

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
                .fill(ThemeColors::BG_PANEL_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(8, 6))
                .corner_radius(3.0_f32)
                .show(ui, |ui| {
                    ui.set_min_width(MINI_HUD_WIDTH);
                    ui.set_max_width(MINI_HUD_WIDTH);
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
                            render_field(ui, shell.hud_state, index, field, &selected, response);
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

fn single_selected_entity(selection: &Selection) -> Option<SelectionItem> {
    if selection.len() != 1 {
        return None;
    }
    selection.iter().find_map(|item| match item {
        SelectionItem::SketchEntity { .. } => Some(item.clone()),
        _ => None,
    })
}

fn sketch_entity_ids(
    item: &SelectionItem,
) -> Option<(roncad_core::ids::SketchId, roncad_core::ids::SketchEntityId)> {
    match item {
        SelectionItem::SketchEntity { sketch, entity } => Some((*sketch, *entity)),
        _ => None,
    }
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

fn ensure_buffers(state: &mut HudEditState, selected: &SelectionItem, fields: &[Field]) {
    let same_entity = state.tracked.as_ref() == Some(selected);
    let same_shape = state.fields.len() == fields.len();
    if !same_entity || !same_shape {
        state.tracked = Some(selected.clone());
        state.fields = fields.iter().map(|f| format_value(f.value)).collect();
        state.focus_index = None;
    }
}

fn render_field(
    ui: &mut Ui,
    state: &mut HudEditState,
    index: usize,
    field: &Field,
    selected: &SelectionItem,
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
                    .background_color(ThemeColors::BG_PANEL_ALT_GLASS)
                    .text_color(ThemeColors::TEXT),
            );

            if text_edit.gained_focus() {
                state.focus_index = Some(index);
                select_all_text(ui, text_edit.id, buffer.chars().count());
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
            if let Some(cmd) = command_for(field.kind, selected, value) {
                response.commands.push(cmd);
            }
        }
        state.focus_index = None;
    }
}

fn command_for(kind: FieldKind, selected: &SelectionItem, value: f64) -> Option<AppCommand> {
    let (sketch, entity) = sketch_entity_ids(selected)?;
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

fn mini_hud_pos(anchor: Pos2, cursor: Option<Pos2>, rect: Rect, field_count: usize) -> Pos2 {
    let size = mini_hud_size(field_count);
    let prefer_left = cursor.is_some_and(|pos| pos.x >= anchor.x);
    let prefer_up = cursor.is_some_and(|pos| pos.y >= anchor.y);

    let left_x = anchor.x - MINI_HUD_GAP - size.x;
    let right_x = anchor.x + MINI_HUD_GAP;
    let up_y = anchor.y - MINI_HUD_GAP - size.y;
    let down_y = anchor.y + MINI_HUD_GAP;

    let min_x = rect.min.x + MINI_HUD_PAD;
    let max_x = rect.max.x - size.x - MINI_HUD_PAD;
    let min_y = rect.min.y + MINI_HUD_PAD;
    let max_y = rect.max.y - size.y - MINI_HUD_PAD;

    let can_left = left_x >= min_x;
    let can_right = right_x <= max_x;
    let can_up = up_y >= min_y;
    let can_down = down_y <= max_y;

    let x = if prefer_left {
        if can_left || !can_right {
            left_x.clamp(min_x, max_x.max(min_x))
        } else {
            right_x.clamp(min_x, max_x.max(min_x))
        }
    } else if can_right || !can_left {
        right_x.clamp(min_x, max_x.max(min_x))
    } else {
        left_x.clamp(min_x, max_x.max(min_x))
    };
    let y = if prefer_up {
        if can_up || !can_down {
            up_y.clamp(min_y, max_y.max(min_y))
        } else {
            down_y.clamp(min_y, max_y.max(min_y))
        }
    } else if can_down || !can_up {
        down_y.clamp(min_y, max_y.max(min_y))
    } else {
        up_y.clamp(min_y, max_y.max(min_y))
    };

    Pos2::new(x, y)
}

fn mini_hud_size(field_count: usize) -> Vec2 {
    Vec2::new(
        MINI_HUD_WIDTH,
        MINI_HUD_BASE_HEIGHT + field_count as f32 * MINI_HUD_ROW_HEIGHT,
    )
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

fn select_all_text(ui: &Ui, widget_id: Id, len: usize) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), widget_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(len))));
    egui::TextEdit::store_state(ui.ctx(), widget_id, state);
}

#[cfg(test)]
mod tests {
    use egui::{pos2, Rect};
    use roncad_core::selection::SelectionItem;

    use super::{
        ensure_buffers, mini_hud_pos, mini_hud_size, select_all_text, Field, FieldKind,
        HudEditState,
    };

    #[test]
    fn ensure_buffers_tracks_selected_item() {
        let mut state = HudEditState::default();
        let selected = SelectionItem::SketchEntity {
            sketch: roncad_core::ids::SketchId::default(),
            entity: roncad_core::ids::SketchEntityId::default(),
        };
        let fields = [Field {
            label: "Length",
            unit: "mm",
            value: 5.0,
            kind: FieldKind::LineLength,
        }];

        ensure_buffers(&mut state, &selected, &fields);

        assert_eq!(state.tracked.as_ref(), Some(&selected));
        assert_eq!(state.fields, vec!["5.000"]);
    }

    #[test]
    fn mini_hud_prefers_opposite_side_of_cursor() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 300.0));
        let anchor = pos2(200.0, 140.0);
        let cursor = Some(pos2(240.0, 180.0));

        let hud_pos = mini_hud_pos(anchor, cursor, rect, 2);
        let size = mini_hud_size(2);

        assert!(hud_pos.x + size.x <= anchor.x);
        assert!(hud_pos.y + size.y <= anchor.y);
    }

    #[test]
    fn mini_hud_clamps_inside_viewport() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(220.0, 160.0));
        let anchor = pos2(210.0, 150.0);

        let hud_pos = mini_hud_pos(anchor, None, rect, 2);
        let size = mini_hud_size(2);

        assert!(hud_pos.x >= 0.0);
        assert!(hud_pos.y >= 0.0);
        assert!(hud_pos.x + size.x <= rect.max.x + 0.1);
        assert!(hud_pos.y + size.y <= rect.max.y + 0.1);
    }

    #[test]
    fn select_all_text_marks_entire_value() {
        egui::__run_test_ui(|ui| {
            let id = egui::Id::new("mini_hud_select_all");

            select_all_text(ui, id, 5);

            let state = egui::TextEdit::load_state(ui.ctx(), id).expect("stored text edit state");
            let range = state.cursor.char_range().expect("selection range");
            assert_eq!(
                range,
                egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(5),
                )
            );
        });
    }
}
