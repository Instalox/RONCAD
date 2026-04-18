//! Applies AppCommand instances to the domain Project. This is the single
//! chokepoint for state mutation; undo/redo will layer on top later.

use roncad_core::command::AppCommand;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{Project, SketchEntity, Sketch};

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
                s.add(SketchEntity::Line { a: *a, b: *b });
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
    use roncad_geometry::Project;

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
}
