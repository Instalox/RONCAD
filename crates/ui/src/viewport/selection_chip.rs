//! Selection-verb chip: a compact floating pill that appears when Select is
//! active and a sketch entity is selected, offering the most relevant next
//! actions. First implementation of "Selection is a verb."
//!
//! Positioned at the top-center of the viewport as a first pass. A later
//! iteration can track the selection centroid or cursor for a truly
//! cursor-adjacent affordance per the manifesto.

use egui::{Area, Frame, Id, Margin, Order, Pos2, Rect, RichText, Sense, Stroke, Ui};
use roncad_core::command::AppCommand;
use roncad_core::constraint::Constraint;
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{Project, SketchEntity};
use roncad_tools::ActiveToolKind;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
) {
    if shell.tool_manager.active_kind() != ActiveToolKind::Select {
        return;
    }
    let Some(suggestion) = suggestion_for(shell.selection, shell.project) else {
        return;
    };

    let anchor = Pos2::new(rect.center().x, rect.min.y + 52.0);
    Area::new(Id::new("viewport_selection_chip"))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .pivot(egui::Align2::CENTER_TOP)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR))
                .inner_margin(Margin::symmetric(8, 4))
                .corner_radius(999.0_f32)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            ThemeColors::TEXT_DIM,
                            RichText::new(&suggestion.hint).size(10.5),
                        );
                        let accent = ThemeColors::tool_accent(ActiveToolKind::Dimension);
                        for button in &suggestion.actions {
                            let label = RichText::new(button.label).size(11.0).strong();
                            let chip = ui.add(
                                egui::Button::new(label)
                                    .fill(accent.gamma_multiply(0.18))
                                    .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)))
                                    .sense(Sense::click()),
                            );
                            if chip.clicked() {
                                match &button.action {
                                    ChipAction::EmitCommand(cmd) => {
                                        response.commands.push(cmd.clone());
                                    }
                                    ChipAction::ActivateTool(kind) => {
                                        shell.tool_manager.set_active(*kind);
                                    }
                                }
                            }
                        }
                    });
                });
        });
}

struct Suggestion {
    hint: String,
    actions: Vec<ChipButton>,
}

struct ChipButton {
    label: &'static str,
    action: ChipAction,
}

enum ChipAction {
    EmitCommand(AppCommand),
    ActivateTool(ActiveToolKind),
}

fn suggestion_for(selection: &Selection, project: &Project) -> Option<Suggestion> {
    let entities: Vec<(SketchId, SketchEntityId)> = selection
        .iter()
        .filter_map(|item| match item {
            SelectionItem::SketchEntity { sketch, entity } => Some((*sketch, *entity)),
            _ => None,
        })
        .collect();

    if entities.is_empty() {
        return None;
    }

    // Single entity: one-click length for a line, otherwise Dimension tool.
    if entities.len() == 1 {
        let (sketch_id, entity_id) = entities[0];
        if let Some(sketch) = project.sketches.get(sketch_id) {
            if let Some(SketchEntity::Line { a, b }) = sketch.entities.get(entity_id) {
                return Some(Suggestion {
                    hint: "1 line".to_string(),
                    actions: vec![ChipButton {
                        label: "Length",
                        action: ChipAction::EmitCommand(AppCommand::AddDistanceDimension {
                            sketch: sketch_id,
                            start: *a,
                            end: *b,
                        }),
                    }],
                });
            }
        }
        return Some(Suggestion {
            hint: "1 selected".to_string(),
            actions: vec![fallback_dimension()],
        });
    }

    // Two entities: offer pairwise relationships if they share a sketch and kind.
    if entities.len() == 2 {
        let (sa, ea) = entities[0];
        let (sb, eb) = entities[1];
        if sa == sb {
            if let Some(sketch) = project.sketches.get(sa) {
                let kind_a = sketch.entities.get(ea).map(entity_kind);
                let kind_b = sketch.entities.get(eb).map(entity_kind);
                match (kind_a, kind_b) {
                    (Some(EntityKind::Line), Some(EntityKind::Line)) => {
                        return Some(Suggestion {
                            hint: "2 lines".to_string(),
                            actions: vec![
                                constraint_button(
                                    "Perp",
                                    sa,
                                    Constraint::Perpendicular { a: ea, b: eb },
                                ),
                                constraint_button(
                                    "Parallel",
                                    sa,
                                    Constraint::Parallel { a: ea, b: eb },
                                ),
                                constraint_button(
                                    "Equal",
                                    sa,
                                    Constraint::EqualLength { a: ea, b: eb },
                                ),
                            ],
                        });
                    }
                    (Some(EntityKind::CircleLike), Some(EntityKind::CircleLike)) => {
                        return Some(Suggestion {
                            hint: "2 circles".to_string(),
                            actions: vec![constraint_button(
                                "Equal radii",
                                sa,
                                Constraint::EqualRadius { a: ea, b: eb },
                            )],
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Fallback: activate the Dimension tool and let the user place it.
    let hint = if entities.len() == 1 {
        "1 selected".to_string()
    } else {
        format!("{} selected", entities.len())
    };
    Some(Suggestion {
        hint,
        actions: vec![fallback_dimension()],
    })
}

fn fallback_dimension() -> ChipButton {
    ChipButton {
        label: "Dimension",
        action: ChipAction::ActivateTool(ActiveToolKind::Dimension),
    }
}

fn constraint_button(label: &'static str, sketch: SketchId, constraint: Constraint) -> ChipButton {
    ChipButton {
        label,
        action: ChipAction::EmitCommand(AppCommand::AddConstraint { sketch, constraint }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityKind {
    Line,
    CircleLike,
    Other,
}

fn entity_kind(entity: &SketchEntity) -> EntityKind {
    match entity {
        SketchEntity::Line { .. } => EntityKind::Line,
        SketchEntity::Circle { .. } | SketchEntity::Arc { .. } => EntityKind::CircleLike,
        _ => EntityKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec2;
    use roncad_geometry::Project;

    fn add_line(project: &mut Project, a: glam::DVec2, b: glam::DVec2) -> SketchEntityId {
        project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line { a, b })
    }

    fn add_circle(project: &mut Project, center: glam::DVec2, radius: f64) -> SketchEntityId {
        project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Circle { center, radius })
    }

    fn select(project: &Project, ids: &[SketchEntityId]) -> Selection {
        let sketch = project.active_sketch.unwrap();
        let mut selection = Selection::default();
        for id in ids {
            selection.insert(SelectionItem::SketchEntity {
                sketch,
                entity: *id,
            });
        }
        selection
    }

    #[test]
    fn empty_selection_has_no_suggestion() {
        let project = Project::new_untitled();
        let selection = Selection::default();
        assert!(suggestion_for(&selection, &project).is_none());
    }

    #[test]
    fn body_only_selection_has_no_suggestion() {
        let project = Project::new_untitled();
        let mut selection = Selection::default();
        selection.insert(SelectionItem::Sketch(project.active_sketch.unwrap()));
        assert!(suggestion_for(&selection, &project).is_none());
    }

    #[test]
    fn single_line_offers_one_click_length() {
        let mut project = Project::new_untitled();
        let line = add_line(&mut project, dvec2(1.0, 2.0), dvec2(7.0, 2.0));
        let s = suggestion_for(&select(&project, &[line]), &project).expect("suggestion");

        assert_eq!(s.hint, "1 line");
        assert_eq!(s.actions.len(), 1);
        assert_eq!(s.actions[0].label, "Length");
        match &s.actions[0].action {
            ChipAction::EmitCommand(AppCommand::AddDistanceDimension { start, end, .. }) => {
                assert_eq!(*start, dvec2(1.0, 2.0));
                assert_eq!(*end, dvec2(7.0, 2.0));
            }
            _ => panic!("expected AddDistanceDimension command"),
        }
    }

    #[test]
    fn single_circle_falls_back_to_dimension_tool() {
        let mut project = Project::new_untitled();
        let c = add_circle(&mut project, dvec2(0.0, 0.0), 3.0);
        let s = suggestion_for(&select(&project, &[c]), &project).expect("suggestion");
        assert_eq!(s.actions.len(), 1);
        assert_eq!(s.actions[0].label, "Dimension");
        assert!(matches!(
            s.actions[0].action,
            ChipAction::ActivateTool(ActiveToolKind::Dimension)
        ));
    }

    #[test]
    fn two_lines_offer_perpendicular_parallel_equal() {
        let mut project = Project::new_untitled();
        let a = add_line(&mut project, dvec2(0.0, 0.0), dvec2(5.0, 0.0));
        let b = add_line(&mut project, dvec2(0.0, 1.0), dvec2(5.0, 1.0));
        let s = suggestion_for(&select(&project, &[a, b]), &project).expect("suggestion");

        assert_eq!(s.hint, "2 lines");
        let labels: Vec<&str> = s.actions.iter().map(|a| a.label).collect();
        assert_eq!(labels, vec!["Perp", "Parallel", "Equal"]);

        let mut seen = [false; 3];
        for btn in &s.actions {
            match &btn.action {
                ChipAction::EmitCommand(AppCommand::AddConstraint { constraint, .. }) => {
                    match constraint {
                        Constraint::Perpendicular { .. } => seen[0] = true,
                        Constraint::Parallel { .. } => seen[1] = true,
                        Constraint::EqualLength { .. } => seen[2] = true,
                        _ => panic!("unexpected constraint kind"),
                    }
                }
                _ => panic!("expected AddConstraint command"),
            }
        }
        assert!(seen.iter().all(|x| *x));
    }

    #[test]
    fn two_circles_offer_equal_radii() {
        let mut project = Project::new_untitled();
        let a = add_circle(&mut project, dvec2(0.0, 0.0), 1.0);
        let b = add_circle(&mut project, dvec2(10.0, 0.0), 2.0);
        let s = suggestion_for(&select(&project, &[a, b]), &project).expect("suggestion");

        assert_eq!(s.hint, "2 circles");
        assert_eq!(s.actions.len(), 1);
        assert_eq!(s.actions[0].label, "Equal radii");
        assert!(matches!(
            &s.actions[0].action,
            ChipAction::EmitCommand(AppCommand::AddConstraint {
                constraint: Constraint::EqualRadius { .. },
                ..
            })
        ));
    }

    #[test]
    fn mixed_line_and_circle_falls_back_to_dimension_tool() {
        let mut project = Project::new_untitled();
        let line = add_line(&mut project, dvec2(0.0, 0.0), dvec2(5.0, 0.0));
        let circle = add_circle(&mut project, dvec2(0.0, 5.0), 2.0);
        let s = suggestion_for(&select(&project, &[line, circle]), &project).expect("suggestion");
        assert_eq!(s.hint, "2 selected");
        assert_eq!(s.actions.len(), 1);
        assert_eq!(s.actions[0].label, "Dimension");
    }

    #[test]
    fn three_lines_fall_back_to_dimension_tool() {
        let mut project = Project::new_untitled();
        let a = add_line(&mut project, dvec2(0.0, 0.0), dvec2(5.0, 0.0));
        let b = add_line(&mut project, dvec2(0.0, 1.0), dvec2(5.0, 1.0));
        let c = add_line(&mut project, dvec2(0.0, 2.0), dvec2(5.0, 2.0));
        let s = suggestion_for(&select(&project, &[a, b, c]), &project).expect("suggestion");
        assert_eq!(s.hint, "3 selected");
        assert_eq!(s.actions[0].label, "Dimension");
    }
}
