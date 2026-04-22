use egui::{Align, Button, Frame, Margin, RichText, Stroke, Ui};
use egui_phosphor::regular as ph;
use roncad_core::{
    constraint::Constraint,
    ids::{ConstraintId, SketchEntityId, SketchId},
    selection::{Selection, SelectionItem},
};
use roncad_geometry::{
    closed_profiles, ConstraintDiagnostic, ConstraintDiagnosticKind, Sketch, SketchDimension,
    SketchEntity, SolveReport, SolveStatus,
};
use slotmap::Key;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

const PREVIEW_ROW_LIMIT: usize = 4;

pub fn render_constraints_section(
    ui: &mut Ui,
    shell: &ShellContext<'_>,
    response: &mut ShellResponse,
) {
    let Some(sketch_id) = shell.project.active_sketch else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("No active sketch.").size(12.0),
        );
        return;
    };
    let Some(sketch) = shell.project.active_sketch() else {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new("No active sketch.").size(12.0),
        );
        return;
    };

    let profile_count = closed_profiles(sketch).len();
    let dimension_count = sketch.dimensions.len();
    let constraint_count = sketch.constraints.len();
    let diagnostics = shell
        .last_solve_report
        .map(|report| report.diagnostics.as_slice())
        .unwrap_or(&[]);
    let selected_entities = selected_sketch_entities(shell.selection, sketch_id);
    let entity_only_selection = selected_entities.len() == shell.selection.len();
    let actions = if entity_only_selection {
        suggested_constraint_actions(sketch, &selected_entities)
    } else {
        Vec::new()
    };

    stat_row(ui, "Sketch", &sketch.name, ThemeColors::TEXT);
    stat_row(
        ui,
        "Selection",
        &format!("{} selected", shell.selection.len()),
        ThemeColors::TEXT_MID,
    );
    stat_row(
        ui,
        "Constraints",
        &constraint_count.to_string(),
        if constraint_count > 0 {
            ThemeColors::ACCENT
        } else {
            ThemeColors::TEXT_MID
        },
    );
    stat_row(
        ui,
        "Dimensions",
        &dimension_count.to_string(),
        if dimension_count > 0 {
            ThemeColors::ACCENT_AMBER
        } else {
            ThemeColors::TEXT_MID
        },
    );
    stat_row(
        ui,
        "Issues",
        &diagnostics.len().to_string(),
        if diagnostics.is_empty() {
            ThemeColors::TEXT_MID
        } else {
            diagnostic_color(Some(diagnostics[0].kind))
        },
    );
    stat_row(ui, "Solver", &solver_label(shell), solver_color(shell));
    if let Some(report) = shell.last_solve_report {
        stat_row(
            ui,
            "Free DOF",
            &report.estimated_free_dofs.to_string(),
            if report.estimated_free_dofs > 0 {
                ThemeColors::ACCENT
            } else {
                ThemeColors::TEXT_MID
            },
        );
    }
    stat_row(
        ui,
        "Closed profiles",
        &profile_count.to_string(),
        if profile_count > 0 {
            ThemeColors::ACCENT
        } else {
            ThemeColors::TEXT_MID
        },
    );

    ui.add_space(8.0);
    ui.colored_label(
        ThemeColors::TEXT_DIM,
        RichText::new("Suggested relations").size(11.0),
    );
    ui.add_space(4.0);
    if actions.is_empty() {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new(if entity_only_selection {
                selection_hint(shell.selection.len())
            } else {
                "Select only sketch entities from the active sketch."
            })
            .size(11.5),
        );
    } else {
        for action in actions {
            constraint_action_row(ui, response, sketch_id, action);
        }
    }

    ui.add_space(8.0);
    if !diagnostics.is_empty() {
        relation_list(
            ui,
            "Problem constraints",
            diagnostic_color(diagnostics.first().map(|item| item.kind)),
            diagnostics.len(),
            |ui| {
                for (index, diagnostic) in diagnostics.iter().take(PREVIEW_ROW_LIMIT).enumerate() {
                    ui.push_id(("constraint_problem_row", index), |ui| {
                        constraint_row(ui, sketch, &diagnostic.constraint, Some(diagnostic));
                    });
                }
            },
        );
        ui.add_space(8.0);
    }

    relation_list(
        ui,
        "Constraints",
        ThemeColors::ACCENT,
        constraint_count,
        |ui| {
            for (index, (constraint_id, constraint)) in sketch
                .iter_constraints()
                .take(PREVIEW_ROW_LIMIT)
                .enumerate()
            {
                ui.push_id(("constraint_row", index), |ui| {
                    constraint_row(
                        ui,
                        sketch,
                        constraint,
                        diagnostic_for_constraint(shell.last_solve_report, constraint_id),
                    );
                });
            }
        },
    );

    ui.add_space(8.0);
    relation_list(
        ui,
        "Dimensions",
        ThemeColors::ACCENT_AMBER,
        dimension_count,
        |ui| {
            for (index, (_, dimension)) in
                sketch.iter_dimensions().take(PREVIEW_ROW_LIMIT).enumerate()
            {
                ui.push_id(("constraint_dimension", index), |ui| {
                    dimension_row(ui, dimension);
                });
            }
        },
    );
}

#[derive(Clone, Copy)]
struct ConstraintAction {
    label: &'static str,
    glyph: &'static str,
    detail: &'static str,
    constraint: Constraint,
    enabled: bool,
}

fn suggested_constraint_actions(
    sketch: &Sketch,
    selected_entities: &[SketchEntityId],
) -> Vec<ConstraintAction> {
    match selected_entities {
        [entity] => single_entity_actions(sketch, *entity),
        [first, second] => pair_entity_actions(sketch, *first, *second),
        _ => Vec::new(),
    }
}

fn single_entity_actions(sketch: &Sketch, entity_id: SketchEntityId) -> Vec<ConstraintAction> {
    match sketch.entities.get(entity_id) {
        Some(SketchEntity::Line { .. }) => vec![
            make_action(
                sketch,
                "Horizontal",
                "H",
                "Constrain selected line horizontal",
                Constraint::Horizontal { entity: entity_id },
            ),
            make_action(
                sketch,
                "Vertical",
                "V",
                "Constrain selected line vertical",
                Constraint::Vertical { entity: entity_id },
            ),
        ],
        _ => Vec::new(),
    }
}

fn pair_entity_actions(
    sketch: &Sketch,
    first: SketchEntityId,
    second: SketchEntityId,
) -> Vec<ConstraintAction> {
    let Some(first_entity) = sketch.entities.get(first) else {
        return Vec::new();
    };
    let Some(second_entity) = sketch.entities.get(second) else {
        return Vec::new();
    };

    match (first_entity, second_entity) {
        (SketchEntity::Line { .. }, SketchEntity::Line { .. }) => vec![
            make_action(
                sketch,
                "Parallel",
                "||",
                "Keep selected lines parallel",
                Constraint::Parallel {
                    a: first,
                    b: second,
                },
            ),
            make_action(
                sketch,
                "Perpendicular",
                "_|_",
                "Make selected lines perpendicular",
                Constraint::Perpendicular {
                    a: first,
                    b: second,
                },
            ),
            make_action(
                sketch,
                "Equal Length",
                "=",
                "Match selected line lengths",
                Constraint::EqualLength {
                    a: first,
                    b: second,
                },
            ),
        ],
        (SketchEntity::Line { .. }, SketchEntity::Circle { .. } | SketchEntity::Arc { .. }) => {
            vec![make_action(
                sketch,
                "Tangent",
                "T",
                "Make the selected line tangent to the curve",
                Constraint::Tangent {
                    line: first,
                    curve: second,
                },
            )]
        }
        (SketchEntity::Circle { .. } | SketchEntity::Arc { .. }, SketchEntity::Line { .. }) => {
            vec![make_action(
                sketch,
                "Tangent",
                "T",
                "Make the selected line tangent to the curve",
                Constraint::Tangent {
                    line: second,
                    curve: first,
                },
            )]
        }
        (
            SketchEntity::Circle { .. } | SketchEntity::Arc { .. },
            SketchEntity::Circle { .. } | SketchEntity::Arc { .. },
        ) => vec![make_action(
            sketch,
            "Equal Radius",
            "R=",
            "Match selected curve radii",
            Constraint::EqualRadius {
                a: first,
                b: second,
            },
        )],
        _ => Vec::new(),
    }
}

fn make_action(
    sketch: &Sketch,
    label: &'static str,
    glyph: &'static str,
    detail: &'static str,
    constraint: Constraint,
) -> ConstraintAction {
    let enabled = !sketch
        .iter_constraints()
        .any(|(_, existing)| *existing == constraint);

    ConstraintAction {
        label,
        glyph,
        detail,
        constraint,
        enabled,
    }
}

fn constraint_action_row(
    ui: &mut Ui,
    response: &mut ShellResponse,
    sketch_id: SketchId,
    action: ConstraintAction,
) {
    let button =
        Button::new(RichText::new(format!("{}  {}", action.glyph, action.label)).size(11.5));
    let mut row = ui.add_enabled(action.enabled, button);
    row = row.on_hover_text(if action.enabled {
        action.detail
    } else {
        "Already applied"
    });
    if row.clicked() {
        response
            .commands
            .push(roncad_core::command::AppCommand::AddConstraint {
                sketch: sketch_id,
                constraint: action.constraint,
            });
    }
}

fn relation_list(
    ui: &mut Ui,
    title: &str,
    accent: egui::Color32,
    count: usize,
    add_rows: impl FnOnce(&mut Ui),
) {
    if count == 0 {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new(format!("No {title} yet.")).size(11.5),
        );
        return;
    }

    ui.horizontal(|ui| {
        ui.colored_label(accent, RichText::new(title).size(11.0));
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new(format!("{count}")).size(10.5).monospace(),
        );
    });
    ui.add_space(3.0);
    add_rows(ui);
    let remaining = count.saturating_sub(PREVIEW_ROW_LIMIT);
    if remaining > 0 {
        ui.colored_label(
            ThemeColors::TEXT_DIM,
            RichText::new(format!("+{remaining} more"))
                .size(10.5)
                .monospace(),
        );
    }
}

fn stat_row(ui: &mut Ui, label: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [84.0, 18.0],
            egui::Label::new(RichText::new(label).size(11.0).color(ThemeColors::TEXT_DIM)),
        );
        ui.colored_label(color, RichText::new(value).size(11.5).monospace());
    });
}

fn constraint_row(
    ui: &mut Ui,
    sketch: &Sketch,
    constraint: &Constraint,
    diagnostic: Option<&ConstraintDiagnostic>,
) {
    let accent = diagnostic_color(diagnostic.map(|item| item.kind));
    let stroke = Stroke::new(
        1.0,
        if diagnostic.is_some() {
            accent.gamma_multiply(0.75)
        } else {
            ThemeColors::SEPARATOR_SOFT
        },
    );
    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(stroke)
        .inner_margin(Margin::symmetric(6, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    accent,
                    RichText::new(constraint_glyph(constraint)).size(11.5),
                );
                ui.label(
                    RichText::new(constraint_label(constraint))
                        .size(11.5)
                        .color(ThemeColors::TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if let Some(diagnostic) = diagnostic {
                        ui.colored_label(
                            accent,
                            RichText::new(format!("r={:.2e}", diagnostic.residual_norm))
                                .size(10.0)
                                .monospace(),
                        );
                        ui.add_space(8.0);
                    }
                    ui.colored_label(
                        ThemeColors::TEXT_DIM,
                        RichText::new(constraint_targets(sketch, constraint))
                            .size(10.5)
                            .monospace(),
                    );
                });
            });
            if let Some(diagnostic) = diagnostic {
                ui.add_space(2.0);
                ui.colored_label(
                    accent,
                    RichText::new(constraint_problem_label(diagnostic.kind)).size(10.5),
                );
            }
        });
}

fn dimension_row(ui: &mut Ui, dimension: &SketchDimension) {
    let (label, value) = match dimension {
        SketchDimension::Distance { start, end } => {
            ("Distance", format!("{:.3} mm", start.distance(*end)))
        }
    };

    Frame::new()
        .fill(ThemeColors::BG_PANEL_ALT)
        .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
        .inner_margin(Margin::symmetric(6, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    ThemeColors::ACCENT_AMBER,
                    RichText::new(ph::RULER).size(11.5),
                );
                ui.label(RichText::new(label).size(11.5).color(ThemeColors::TEXT));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.colored_label(
                        ThemeColors::ACCENT_AMBER,
                        RichText::new(value).size(11.0).monospace(),
                    );
                });
            });
        });
}

fn selected_sketch_entities(selection: &Selection, sketch_id: SketchId) -> Vec<SketchEntityId> {
    let mut entities = selection
        .iter()
        .filter_map(|item| match item {
            SelectionItem::SketchEntity { sketch, entity } if *sketch == sketch_id => Some(*entity),
            _ => None,
        })
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity_id| entity_slot(*entity_id));
    entities
}

fn selection_hint(selection_len: usize) -> &'static str {
    match selection_len {
        0 => "Select geometry to add explicit relations.",
        1 => "Select one line for Horizontal or Vertical.",
        2 => "Select two lines, two curves, or a line and a curve.",
        _ => "Reduce selection to one or two sketch entities.",
    }
}

fn solver_label(shell: &ShellContext<'_>) -> String {
    match shell.last_solve_report {
        Some(report) => match report.status {
            SolveStatus::Underdefined => "Underdefined".to_string(),
            SolveStatus::Solved => "Solved".to_string(),
            SolveStatus::Conflicting => "Conflicting".to_string(),
            SolveStatus::Failed => "Failed".to_string(),
        },
        None => "Idle".to_string(),
    }
}

fn solver_color(shell: &ShellContext<'_>) -> egui::Color32 {
    match shell.last_solve_report.map(|report| report.status) {
        Some(SolveStatus::Underdefined) => ThemeColors::ACCENT,
        Some(SolveStatus::Solved) => ThemeColors::ACCENT_GREEN,
        Some(SolveStatus::Conflicting) => ThemeColors::ACCENT_AMBER,
        Some(SolveStatus::Failed) => ThemeColors::ACCENT_RED,
        None => ThemeColors::TEXT_DIM,
    }
}

fn diagnostic_for_constraint<'a>(
    report: Option<&'a SolveReport>,
    constraint_id: ConstraintId,
) -> Option<&'a ConstraintDiagnostic> {
    report?
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == constraint_id)
}

fn diagnostic_color(kind: Option<ConstraintDiagnosticKind>) -> egui::Color32 {
    match kind {
        Some(ConstraintDiagnosticKind::Unsatisfied) => ThemeColors::ACCENT_AMBER,
        Some(ConstraintDiagnosticKind::Failed) => ThemeColors::ACCENT_RED,
        None => ThemeColors::ACCENT,
    }
}

fn constraint_problem_label(kind: ConstraintDiagnosticKind) -> &'static str {
    match kind {
        ConstraintDiagnosticKind::Unsatisfied => "Unsatisfied after solve",
        ConstraintDiagnosticKind::Failed => "Could not evaluate",
    }
}

fn constraint_label(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::Coincident { .. } => "Coincident",
        Constraint::PointOnEntity { .. } => "Point On Entity",
        Constraint::Horizontal { .. } => "Horizontal",
        Constraint::Vertical { .. } => "Vertical",
        Constraint::Parallel { .. } => "Parallel",
        Constraint::Perpendicular { .. } => "Perpendicular",
        Constraint::Tangent { .. } => "Tangent",
        Constraint::EqualLength { .. } => "Equal Length",
        Constraint::EqualRadius { .. } => "Equal Radius",
    }
}

fn constraint_glyph(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::Coincident { .. } => "o",
        Constraint::PointOnEntity { .. } => "O",
        Constraint::Horizontal { .. } => "H",
        Constraint::Vertical { .. } => "V",
        Constraint::Parallel { .. } => "||",
        Constraint::Perpendicular { .. } => "_|_",
        Constraint::Tangent { .. } => "T",
        Constraint::EqualLength { .. } => "=",
        Constraint::EqualRadius { .. } => "R=",
    }
}

fn constraint_targets(sketch: &Sketch, constraint: &Constraint) -> String {
    match constraint {
        Constraint::Coincident { a, b } => format!(
            "{} · {}",
            entity_tag(a.entity(), sketch),
            entity_tag(b.entity(), sketch)
        ),
        Constraint::PointOnEntity { point, entity } => format!(
            "{} · {}",
            entity_tag(point.entity(), sketch),
            entity_tag(*entity, sketch)
        ),
        Constraint::Horizontal { entity } | Constraint::Vertical { entity } => {
            entity_tag(*entity, sketch)
        }
        Constraint::Parallel { a, b }
        | Constraint::Perpendicular { a, b }
        | Constraint::EqualLength { a, b }
        | Constraint::EqualRadius { a, b } => {
            format!("{} · {}", entity_tag(*a, sketch), entity_tag(*b, sketch))
        }
        Constraint::Tangent { line, curve } => {
            format!(
                "{} · {}",
                entity_tag(*line, sketch),
                entity_tag(*curve, sketch)
            )
        }
    }
}

fn entity_tag(entity_id: SketchEntityId, sketch: &Sketch) -> String {
    let prefix = match sketch.entities.get(entity_id) {
        Some(entity) => match entity {
            SketchEntity::Point { .. } => "P",
            SketchEntity::Line { .. } => "L",
            SketchEntity::Rectangle { .. } => "R",
            SketchEntity::Circle { .. } => "C",
            SketchEntity::Arc { .. } => "A",
        },
        None => "?",
    };
    format!("{prefix}{:03}", entity_slot(entity_id))
}

fn entity_slot(entity_id: SketchEntityId) -> u32 {
    (entity_id.data().as_ffi() & 0xffff_ffff) as u32
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::selection::{Selection, SelectionItem};
    use roncad_geometry::{Constraint, Project, SketchEntity};

    use super::{selected_sketch_entities, suggested_constraint_actions};

    #[test]
    fn single_line_suggests_horizontal_and_vertical() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 3.0),
        });

        let actions = suggested_constraint_actions(sketch, &[line]);

        assert_eq!(actions.len(), 2);
        assert!(actions
            .iter()
            .any(|action| matches!(action.constraint, Constraint::Horizontal { entity } if entity == line)));
        assert!(actions.iter().any(
            |action| matches!(action.constraint, Constraint::Vertical { entity } if entity == line)
        ));
    }

    #[test]
    fn two_lines_suggest_parallel_perpendicular_and_equal_length() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let first = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(5.0, 0.0),
        });
        let second = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 4.0),
            b: dvec2(5.0, 2.0),
        });

        let actions = suggested_constraint_actions(sketch, &[first, second]);

        assert_eq!(actions.len(), 3);
        assert!(actions
            .iter()
            .any(|action| matches!(action.constraint, Constraint::Parallel { a, b } if a == first && b == second)));
        assert!(actions
            .iter()
            .any(|action| matches!(action.constraint, Constraint::Perpendicular { a, b } if a == first && b == second)));
        assert!(actions
            .iter()
            .any(|action| matches!(action.constraint, Constraint::EqualLength { a, b } if a == first && b == second)));
    }

    #[test]
    fn line_and_circle_suggest_tangent() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(5.0, 0.0),
        });
        let circle = sketch.add(SketchEntity::Circle {
            center: dvec2(2.0, 3.0),
            radius: 1.0,
        });

        let actions = suggested_constraint_actions(sketch, &[circle, line]);

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0].constraint,
            Constraint::Tangent { line: tangent_line, curve } if tangent_line == line && curve == circle
        ));
    }

    #[test]
    fn existing_constraint_is_disabled() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(5.0, 0.0),
        });
        sketch.add_constraint(Constraint::Horizontal { entity: line });

        let actions = suggested_constraint_actions(sketch, &[line]);

        assert!(actions
            .iter()
            .find(|action| matches!(action.constraint, Constraint::Horizontal { entity } if entity == line))
            .is_some_and(|action| !action.enabled));
    }

    #[test]
    fn selected_sketch_entities_ignores_non_matching_items() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("sketch");
        let other_sketch = project.sketches.insert(roncad_geometry::Sketch::new(
            "Other",
            project.workplanes.keys().next().unwrap(),
        ));
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let line = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(1.0, 0.0),
        });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity: line,
        });
        selection.insert(SelectionItem::Sketch(other_sketch));

        let selected = selected_sketch_entities(&selection, sketch_id);

        assert_eq!(selected, vec![line]);
    }
}
