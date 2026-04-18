//! Applies AppCommand instances to the domain Project. This is the single
//! chokepoint for state mutation; undo/redo will layer on top later.

use roncad_core::command::AppCommand;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{Project, Sketch, SketchDimension, SketchEntity};

pub fn apply(
    project: &mut Project,
    selection: &mut Selection,
    command: &AppCommand,
) {
    match command {
        AppCommand::CreateSketch { name } => {
            if let Some(plane_id) = project.workplanes.keys().next() {
                let id = project.sketches.insert(Sketch::new(name, plane_id));
                project.active_sketch = Some(id);
                selection.clear();
            }
        }
        AppCommand::SetActiveSketch(id) => {
            if project.sketches.contains_key(*id) {
                project.active_sketch = Some(*id);
                selection.clear();
            }
        }
        AppCommand::DeleteSketch(id) => {
            project.sketches.remove(*id);
            selection.retain(|item| match item {
                SelectionItem::Sketch(sketch) => sketch != id,
                SelectionItem::SketchEntity { sketch, .. } => sketch != id,
                SelectionItem::Body(_) => true,
            });
            if project.active_sketch == Some(*id) {
                project.active_sketch = project.sketches.keys().next();
            }
        }
        AppCommand::AddLine { sketch, a, b } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let result = s.add_line_with_splits(*a, *b);
                for replacement in result.replaced {
                    let selected_original = SelectionItem::SketchEntity {
                        sketch: *sketch,
                        entity: replacement.original,
                    };
                    if selection.remove(&selected_original) {
                        for entity in replacement.segments {
                            selection.insert(SelectionItem::SketchEntity {
                                sketch: *sketch,
                                entity,
                            });
                        }
                    }
                }
            }
        }
        AppCommand::AddRectangle { sketch, corner_a, corner_b } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                s.add(SketchEntity::Rectangle {
                    corner_a: *corner_a,
                    corner_b: *corner_b,
                });
            }
        }
        AppCommand::AddCircle { sketch, center, radius } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                s.add(SketchEntity::Circle {
                    center: *center,
                    radius: radius.as_f64(),
                });
            }
        }
        AppCommand::AddDistanceDimension { sketch, start, end } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                s.add_dimension(SketchDimension::Distance {
                    start: *start,
                    end: *end,
                });
            }
        }
        AppCommand::DeleteEntity { sketch, entity } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                s.remove(*entity);
            }
            selection.remove(&SelectionItem::SketchEntity {
                sketch: *sketch,
                entity: *entity,
            });
        }
        AppCommand::SelectSingle { sketch, entity } => {
            let exists = project
                .sketches
                .get(*sketch)
                .is_some_and(|s| s.entities.contains_key(*entity));
            if exists {
                selection.clear();
                selection.insert(SelectionItem::SketchEntity {
                    sketch: *sketch,
                    entity: *entity,
                });
            } else {
                selection.clear();
            }
        }
        AppCommand::ToggleSelection { sketch, entity } => {
            let item = SelectionItem::SketchEntity {
                sketch: *sketch,
                entity: *entity,
            };
            let exists = project
                .sketches
                .get(*sketch)
                .is_some_and(|s| s.entities.contains_key(*entity));
            if !exists {
                selection.remove(&item);
            } else if !selection.remove(&item) {
                selection.insert(item);
            }
        }
        AppCommand::ClearSelection => {
            selection.clear();
        }
        AppCommand::DeleteSelection => {
            let selected_entities: Vec<_> = selection
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::SketchEntity { sketch, entity } => Some((*sketch, *entity)),
                    _ => None,
                })
                .collect();

            for (sketch, entity) in selected_entities {
                if let Some(s) = project.sketches.get_mut(sketch) {
                    s.remove(entity);
                }
                selection.remove(&SelectionItem::SketchEntity { sketch, entity });
            }
        }
        AppCommand::ExtrudeProfile { .. } => {
            tracing::debug!("ExtrudeProfile not yet implemented");
        }
        AppCommand::NoOp => {}
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::selection::SelectionItem;
    use roncad_geometry::{Project, Sketch, SketchDimension};

    use super::apply;
    use roncad_core::command::AppCommand;
    use roncad_core::selection::Selection;
    use roncad_geometry::SketchEntity;

    #[test]
    fn select_single_replaces_previous_selection() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });

        selection.insert(SelectionItem::Sketch(sketch));
        apply(
            &mut project,
            &mut selection,
            &AppCommand::SelectSingle { sketch, entity },
        );

        assert_eq!(selection.len(), 1);
        assert!(selection.contains(&SelectionItem::SketchEntity { sketch, entity }));
    }

    #[test]
    fn delete_selection_removes_selected_entities() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Circle {
                center: dvec2(5.0, 5.0),
                radius: 3.0,
            });

        selection.insert(SelectionItem::SketchEntity { sketch, entity });
        apply(&mut project, &mut selection, &AppCommand::DeleteSelection);

        assert!(selection.is_empty());
        assert!(
            !project
                .active_sketch()
                .expect("active sketch")
                .entities
                .contains_key(entity)
        );
    }

    #[test]
    fn set_active_sketch_switches_context_and_clears_selection() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let first = project.active_sketch.expect("default project has sketch");
        let plane = project.workplanes.keys().next().expect("default plane");
        let second = project.sketches.insert(Sketch::new("Sketch 2", plane));
        selection.insert(SelectionItem::Sketch(first));

        apply(
            &mut project,
            &mut selection,
            &AppCommand::SetActiveSketch(second),
        );

        assert_eq!(project.active_sketch, Some(second));
        assert!(selection.is_empty());
    }

    #[test]
    fn add_distance_dimension_persists_on_sketch() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");

        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddDistanceDimension {
                sketch,
                start: dvec2(1.0, 2.0),
                end: dvec2(6.0, 2.0),
            },
        );

        let dimensions: Vec<_> = project
            .active_sketch()
            .expect("active sketch")
            .iter_dimensions()
            .collect();
        assert_eq!(dimensions.len(), 1);
        assert!(matches!(
            dimensions[0].1,
            SketchDimension::Distance { start, end }
                if *start == dvec2(1.0, 2.0) && *end == dvec2(6.0, 2.0)
        ));
    }

    #[test]
    fn add_line_splits_crossing_lines_into_four_entities() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");

        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 10.0),
            },
        );
        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 10.0),
                b: dvec2(10.0, 0.0),
            },
        );

        let lines: Vec<_> = project
            .active_sketch()
            .expect("active sketch")
            .iter()
            .filter_map(|(_, entity)| match entity {
                SketchEntity::Line { a, b } => Some((*a, *b)),
                _ => None,
            })
            .collect();

        assert_eq!(lines.len(), 4);
        assert!(contains_line(&lines, dvec2(0.0, 0.0), dvec2(5.0, 5.0)));
        assert!(contains_line(&lines, dvec2(5.0, 5.0), dvec2(10.0, 10.0)));
        assert!(contains_line(&lines, dvec2(0.0, 10.0), dvec2(5.0, 5.0)));
        assert!(contains_line(&lines, dvec2(5.0, 5.0), dvec2(10.0, 0.0)));
    }

    #[test]
    fn split_line_replaces_existing_selection_with_new_segments() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let original = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        selection.insert(SelectionItem::SketchEntity {
            sketch,
            entity: original,
        });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddLine {
                sketch,
                a: dvec2(5.0, -4.0),
                b: dvec2(5.0, 4.0),
            },
        );

        let selected_lines: Vec<_> = selection
            .iter()
            .filter_map(|item| match item {
                SelectionItem::SketchEntity { sketch: selected_sketch, entity }
                    if *selected_sketch == sketch =>
                {
                    match project
                        .active_sketch()
                        .expect("active sketch")
                        .entities
                        .get(*entity)
                    {
                        Some(SketchEntity::Line { a, b }) => Some((*a, *b)),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();

        assert_eq!(selected_lines.len(), 2);
        assert!(contains_line(
            &selected_lines,
            dvec2(0.0, 0.0),
            dvec2(5.0, 0.0)
        ));
        assert!(contains_line(
            &selected_lines,
            dvec2(5.0, 0.0),
            dvec2(10.0, 0.0)
        ));
    }

    fn contains_line(lines: &[(glam::DVec2, glam::DVec2)], a: glam::DVec2, b: glam::DVec2) -> bool {
        lines.iter().any(|(line_a, line_b)| {
            (*line_a == a && *line_b == b) || (*line_a == b && *line_b == a)
        })
    }
}
