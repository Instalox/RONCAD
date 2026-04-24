//! Selection-verb chip: a compact floating pill that appears when Select is
//! active and a sketch entity is selected, offering the most relevant next
//! action. First implementation of "Selection is a verb."
//!
//! Positioned at the top-center of the viewport as a first pass. A later
//! iteration can track the selection centroid or cursor for a truly
//! cursor-adjacent affordance per the manifesto.

use egui::{Area, Frame, Id, Margin, Order, Pos2, Rect, RichText, Sense, Stroke, Ui};
use roncad_core::command::AppCommand;
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
                            RichText::new(suggestion.hint).size(10.5),
                        );
                        let accent = ThemeColors::tool_accent(ActiveToolKind::Dimension);
                        let label = RichText::new(suggestion.label).size(11.0).strong();
                        let chip = ui.add(
                            egui::Button::new(label)
                                .fill(accent.gamma_multiply(0.18))
                                .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)))
                                .sense(Sense::click()),
                        );
                        if chip.clicked() {
                            match suggestion.action {
                                ChipAction::EmitCommand(ref cmd) => {
                                    response.commands.push(cmd.clone());
                                }
                                ChipAction::ActivateTool(kind) => {
                                    shell.tool_manager.set_active(kind);
                                }
                            }
                        }
                    });
                });
        });
}

struct Suggestion {
    hint: String,
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

    // Single-Line selection: one-click length dimension.
    if entities.len() == 1 {
        let (sketch_id, entity_id) = entities[0];
        if let Some(sketch) = project.sketches.get(sketch_id) {
            if let Some(SketchEntity::Line { a, b }) = sketch.entities.get(entity_id) {
                return Some(Suggestion {
                    hint: "1 line".to_string(),
                    label: "Length",
                    action: ChipAction::EmitCommand(AppCommand::AddDistanceDimension {
                        sketch: sketch_id,
                        start: *a,
                        end: *b,
                    }),
                });
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
        label: "Dimension",
        action: ChipAction::ActivateTool(ActiveToolKind::Dimension),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec2;
    use roncad_geometry::Project;

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
        let sketch_id = project.active_sketch.unwrap();
        let entity = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(1.0, 2.0),
                b: dvec2(7.0, 2.0),
            });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity,
        });

        let s = suggestion_for(&selection, &project).expect("suggestion");
        assert_eq!(s.label, "Length");
        assert_eq!(s.hint, "1 line");
        match s.action {
            ChipAction::EmitCommand(AppCommand::AddDistanceDimension { start, end, .. }) => {
                assert_eq!(start, dvec2(1.0, 2.0));
                assert_eq!(end, dvec2(7.0, 2.0));
            }
            _ => panic!("expected AddDistanceDimension command"),
        }
    }

    #[test]
    fn single_circle_falls_back_to_dimension_tool() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.unwrap();
        let entity = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Circle {
                center: dvec2(0.0, 0.0),
                radius: 3.0,
            });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity,
        });

        let s = suggestion_for(&selection, &project).expect("suggestion");
        assert_eq!(s.label, "Dimension");
        assert!(matches!(
            s.action,
            ChipAction::ActivateTool(ActiveToolKind::Dimension)
        ));
    }

    #[test]
    fn multi_selection_falls_back_to_dimension_tool() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.unwrap();
        let a = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 0.0),
            });
        let b = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(0.0, 1.0),
                b: dvec2(5.0, 1.0),
            });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity: a,
        });
        selection.insert(SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity: b,
        });

        let s = suggestion_for(&selection, &project).expect("suggestion");
        assert_eq!(s.hint, "2 selected");
        assert!(matches!(
            s.action,
            ChipAction::ActivateTool(ActiveToolKind::Dimension)
        ));
    }

}
