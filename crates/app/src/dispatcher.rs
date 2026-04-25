//! Applies AppCommand instances to the domain Project. This is the single
//! chokepoint for state mutation; undo/redo will layer on top later.

use glam::DVec2;
use roncad_core::command::{AppCommand, ProfileRegion, SelectionEditMode};
use roncad_core::constraint::{Constraint, EntityPoint};
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{
    apply_line_fillet, infer_constraints, resolve_entity_point, solve_sketch, Project, Sketch,
    SketchDimension, SketchEntity, SketchProfile, SolveReport,
};

pub fn apply(
    project: &mut Project,
    selection: &mut Selection,
    command: &AppCommand,
) -> Option<SolveReport> {
    let resolve_target = sketch_target(command);
    match command {
        AppCommand::CreateSketch { name, plane } => {
            if project.workplanes.contains_key(*plane) {
                let id = project.sketches.insert(Sketch::new(name, *plane));
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
            project.clear_feature_sketch_source(*id);
            selection.retain(|item| match item {
                SelectionItem::Sketch(sketch) => sketch != id,
                SelectionItem::SketchEntity { sketch, .. } => sketch != id,
                SelectionItem::SketchVertex { sketch, .. } => sketch != id,
                SelectionItem::Body(_) => true,
            });
            if project.active_sketch == Some(*id) {
                project.active_sketch = project.sketches.keys().next();
            }
        }
        AppCommand::AddLine { sketch, a, b } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let id = s.add(SketchEntity::Line { a: *a, b: *b });
                infer_constraints(s, id);
            }
        }
        AppCommand::AddRectangle {
            sketch,
            corner_a,
            corner_b,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                add_canonical_rectangle(s, *corner_a, *corner_b);
            }
        }
        AppCommand::AddCircle {
            sketch,
            center,
            radius,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let id = s.add(SketchEntity::Circle {
                    center: *center,
                    radius: radius.as_f64(),
                });
                infer_constraints(s, id);
            }
        }
        AppCommand::AddArc {
            sketch,
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let id = s.add(SketchEntity::Arc {
                    center: *center,
                    radius: radius.as_f64(),
                    start_angle: *start_angle,
                    sweep_angle: *sweep_angle,
                });
                infer_constraints(s, id);
            }
        }
        AppCommand::ApplyLineFillet {
            sketch,
            line_a,
            line_b,
            corner,
            radius,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let selected_a = selection.contains(&SelectionItem::SketchEntity {
                    sketch: *sketch,
                    entity: *line_a,
                });
                let selected_b = selection.contains(&SelectionItem::SketchEntity {
                    sketch: *sketch,
                    entity: *line_b,
                });

                if let Some(result) =
                    apply_line_fillet(s, *line_a, *line_b, *corner, radius.as_f64())
                {
                    if selected_a || selected_b {
                        for removed in result.removed.into_iter().flatten() {
                            selection.remove(&SelectionItem::SketchEntity {
                                sketch: *sketch,
                                entity: removed,
                            });
                        }

                        for entity in result.inserted_lines {
                            selection.insert(SelectionItem::SketchEntity {
                                sketch: *sketch,
                                entity,
                            });
                        }
                        if let Some(entity) = result.inserted_arc {
                            selection.insert(SelectionItem::SketchEntity {
                                sketch: *sketch,
                                entity,
                            });
                        }
                    }
                }
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
        AppCommand::AddConstraint { sketch, constraint } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                let refs = constraint.referenced_entities();
                if refs.iter().all(|id| s.entities.contains_key(*id))
                    && !s
                        .iter_constraints()
                        .any(|(_, existing)| existing == constraint)
                {
                    s.add_constraint(*constraint);
                }
            }
        }
        AppCommand::RemoveConstraint { sketch, constraint } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                s.remove_constraint(*constraint);
            }
        }
        AppCommand::SetLineLength {
            sketch,
            entity,
            length,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Line { a, b }) = s.entities.get_mut(*entity) {
                    let new_len = length.as_f64().max(0.0);
                    let delta = *b - *a;
                    let dir = if delta.length_squared() > 1e-20 {
                        delta.normalize()
                    } else {
                        glam::DVec2::X
                    };
                    *b = *a + dir * new_len;
                }
            }
        }
        AppCommand::SetRectangleWidth {
            sketch,
            entity,
            width,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Rectangle { corner_a, corner_b }) =
                    s.entities.get_mut(*entity)
                {
                    let sign = if corner_b.x >= corner_a.x { 1.0 } else { -1.0 };
                    corner_b.x = corner_a.x + sign * width.as_f64().max(0.0);
                }
            }
        }
        AppCommand::SetRectangleHeight {
            sketch,
            entity,
            height,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Rectangle { corner_a, corner_b }) =
                    s.entities.get_mut(*entity)
                {
                    let sign = if corner_b.y >= corner_a.y { 1.0 } else { -1.0 };
                    corner_b.y = corner_a.y + sign * height.as_f64().max(0.0);
                }
            }
        }
        AppCommand::SetCircleRadius {
            sketch,
            entity,
            radius,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Circle { radius: r, .. }) = s.entities.get_mut(*entity) {
                    *r = radius.as_f64().max(0.0);
                }
            }
        }
        AppCommand::SetArcRadius {
            sketch,
            entity,
            radius,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Arc { radius: r, .. }) = s.entities.get_mut(*entity) {
                    *r = radius.as_f64().max(0.0);
                }
            }
        }
        AppCommand::SetArcSweepDegrees {
            sketch,
            entity,
            sweep_degrees,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Arc { sweep_angle, .. }) = s.entities.get_mut(*entity) {
                    *sweep_angle = sweep_degrees.to_radians();
                }
            }
        }
        AppCommand::SetPointX { sketch, entity, x } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Point { p }) = s.entities.get_mut(*entity) {
                    p.x = *x;
                }
            }
        }
        AppCommand::SetPointY { sketch, entity, y } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                if let Some(SketchEntity::Point { p }) = s.entities.get_mut(*entity) {
                    p.y = *y;
                }
            }
        }
        AppCommand::TranslateSketchSelection {
            sketch,
            vertices,
            entities,
            delta,
        } => {
            if let Some(s) = project.sketches.get_mut(*sketch) {
                translate_sketch_selection(s, vertices, entities, *delta);
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
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::Point(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::Start(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::End(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::Center(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::CornerA(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::CornerB(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::CornerC(*entity),
            });
            selection.remove(&SelectionItem::SketchVertex {
                sketch: *sketch,
                point: EntityPoint::CornerD(*entity),
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
        AppCommand::SelectEntities {
            sketch,
            entities,
            mode,
        } => {
            let Some(sketch_doc) = project.sketches.get(*sketch) else {
                selection.clear();
                return None;
            };
            let valid_entities: Vec<_> = entities
                .iter()
                .copied()
                .filter(|entity| sketch_doc.entities.contains_key(*entity))
                .collect();

            if *mode == SelectionEditMode::Replace {
                selection.clear();
            }

            for entity in valid_entities {
                let item = SelectionItem::SketchEntity {
                    sketch: *sketch,
                    entity,
                };
                match mode {
                    SelectionEditMode::Replace | SelectionEditMode::Add => {
                        selection.insert(item);
                    }
                    SelectionEditMode::Remove => {
                        selection.remove(&item);
                    }
                    SelectionEditMode::Toggle => {
                        if !selection.remove(&item) {
                            selection.insert(item);
                        }
                    }
                }
            }
        }
        AppCommand::SelectVertices {
            sketch,
            points,
            mode,
        } => {
            let Some(sketch_doc) = project.sketches.get(*sketch) else {
                selection.clear();
                return None;
            };
            let valid_points: Vec<_> = points
                .iter()
                .copied()
                .filter(|point| {
                    sketch_doc
                        .entities
                        .get(point.entity())
                        .and_then(|entity| resolve_entity_point(*point, entity))
                        .is_some()
                })
                .collect();

            if *mode == SelectionEditMode::Replace {
                selection.clear();
            }

            for point in valid_points {
                let item = SelectionItem::SketchVertex {
                    sketch: *sketch,
                    point,
                };
                match mode {
                    SelectionEditMode::Replace | SelectionEditMode::Add => {
                        selection.insert(item);
                    }
                    SelectionEditMode::Remove => {
                        selection.remove(&item);
                    }
                    SelectionEditMode::Toggle => {
                        if !selection.remove(&item) {
                            selection.insert(item);
                        }
                    }
                }
            }
        }
        AppCommand::SelectBody(body) => {
            if project.bodies.contains_key(*body) {
                selection.clear();
                selection.insert(SelectionItem::Body(*body));
            } else {
                selection.clear();
            }
        }
        AppCommand::SelectBodies { bodies, mode } => {
            let valid_bodies: Vec<_> = bodies
                .iter()
                .copied()
                .filter(|body| project.bodies.contains_key(*body))
                .collect();

            if *mode == SelectionEditMode::Replace {
                selection.clear();
            }

            for body in valid_bodies {
                let item = SelectionItem::Body(body);
                match mode {
                    SelectionEditMode::Replace | SelectionEditMode::Add => {
                        selection.insert(item);
                    }
                    SelectionEditMode::Remove => {
                        selection.remove(&item);
                    }
                    SelectionEditMode::Toggle => {
                        if !selection.remove(&item) {
                            selection.insert(item);
                        }
                    }
                }
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
            let selected_bodies: Vec<_> = selection
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::Body(body) => Some(*body),
                    _ => None,
                })
                .collect();
            let selected_vertices: Vec<_> = selection
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::SketchVertex { sketch, point } => Some((*sketch, *point)),
                    _ => None,
                })
                .collect();

            for (sketch, entity) in selected_entities {
                if let Some(s) = project.sketches.get_mut(sketch) {
                    s.remove(entity);
                }
                selection.remove(&SelectionItem::SketchEntity { sketch, entity });
            }
            for body in selected_bodies {
                project.delete_body(body);
                selection.remove(&SelectionItem::Body(body));
            }
            for (sketch, point) in selected_vertices {
                selection.remove(&SelectionItem::SketchVertex { sketch, point });
            }
        }
        AppCommand::ExtrudeProfile {
            sketch,
            profile,
            distance,
        } => {
            if let Some((body, _feature)) =
                project.extrude_profile(*sketch, sketch_profile(profile), distance.as_f64())
            {
                selection.clear();
                selection.insert(SelectionItem::Body(body));
            }
        }
        AppCommand::RevolveProfile {
            sketch,
            profile,
            axis_origin,
            axis_dir,
            angle_rad,
        } => {
            if let Some((body, _feature)) = project.revolve_profile(
                *sketch,
                sketch_profile(profile),
                *axis_origin,
                *axis_dir,
                *angle_rad,
            ) {
                selection.clear();
                selection.insert(SelectionItem::Body(body));
            }
        }
        AppCommand::NoOp => {}
    }

    if let Some(sketch_id) = resolve_target {
        let solve_report = project.sketches.get_mut(sketch_id).map(solve_sketch);
        project.rebuild_features_for_sketch(sketch_id);
        return solve_report;
    }

    None
}

/// Sketch id to re-solve after this command runs, if any. Commands that
/// mutate entity geometry or the constraint set get the solver; selection
/// and feature-producing commands skip it.
fn sketch_target(command: &AppCommand) -> Option<SketchId> {
    match command {
        AppCommand::AddLine { sketch, .. }
        | AppCommand::AddRectangle { sketch, .. }
        | AppCommand::AddCircle { sketch, .. }
        | AppCommand::AddArc { sketch, .. }
        | AppCommand::ApplyLineFillet { sketch, .. }
        | AppCommand::AddConstraint { sketch, .. }
        | AppCommand::RemoveConstraint { sketch, .. }
        | AppCommand::SetLineLength { sketch, .. }
        | AppCommand::SetRectangleWidth { sketch, .. }
        | AppCommand::SetRectangleHeight { sketch, .. }
        | AppCommand::SetCircleRadius { sketch, .. }
        | AppCommand::SetArcRadius { sketch, .. }
        | AppCommand::SetArcSweepDegrees { sketch, .. }
        | AppCommand::SetPointX { sketch, .. }
        | AppCommand::SetPointY { sketch, .. }
        | AppCommand::TranslateSketchSelection { sketch, .. }
        | AppCommand::DeleteEntity { sketch, .. } => Some(*sketch),
        _ => None,
    }
}

fn add_canonical_rectangle(
    sketch: &mut Sketch,
    corner_a: DVec2,
    corner_c: DVec2,
) -> [SketchEntityId; 4] {
    let corner_b = DVec2::new(corner_c.x, corner_a.y);
    let corner_d = DVec2::new(corner_a.x, corner_c.y);

    let bottom = sketch.add(SketchEntity::Line {
        a: corner_a,
        b: corner_b,
    });
    let right = sketch.add(SketchEntity::Line {
        a: corner_b,
        b: corner_c,
    });
    let top = sketch.add(SketchEntity::Line {
        a: corner_c,
        b: corner_d,
    });
    let left = sketch.add(SketchEntity::Line {
        a: corner_d,
        b: corner_a,
    });

    for constraint in [
        Constraint::Horizontal { entity: bottom },
        Constraint::Vertical { entity: right },
        Constraint::Horizontal { entity: top },
        Constraint::Vertical { entity: left },
        Constraint::Coincident {
            a: EntityPoint::End(bottom),
            b: EntityPoint::Start(right),
        },
        Constraint::Coincident {
            a: EntityPoint::End(right),
            b: EntityPoint::Start(top),
        },
        Constraint::Coincident {
            a: EntityPoint::End(top),
            b: EntityPoint::Start(left),
        },
        Constraint::Coincident {
            a: EntityPoint::End(left),
            b: EntityPoint::Start(bottom),
        },
    ] {
        add_constraint_once(sketch, constraint);
    }

    for line in [bottom, right, top, left] {
        infer_constraints(sketch, line);
    }

    [bottom, right, top, left]
}

fn translate_sketch_selection(
    sketch: &mut Sketch,
    vertices: &[EntityPoint],
    entities: &[SketchEntityId],
    delta: DVec2,
) {
    if delta.length_squared() <= f64::EPSILON || !delta.is_finite() {
        return;
    }

    let entity_set: std::collections::HashSet<_> = entities.iter().copied().collect();
    for entity in &entity_set {
        translate_entity(sketch, *entity, delta);
    }

    let mut moved_handles = std::collections::HashSet::new();
    for vertex in vertices {
        if entity_set.contains(&vertex.entity()) || !moved_handles.insert(*vertex) {
            continue;
        }
        translate_vertex(sketch, *vertex, delta);
    }
}

fn translate_entity(sketch: &mut Sketch, entity: SketchEntityId, delta: DVec2) {
    let Some(entity) = sketch.entities.get_mut(entity) else {
        return;
    };
    match entity {
        SketchEntity::Point { p } => *p += delta,
        SketchEntity::Line { a, b } => {
            *a += delta;
            *b += delta;
        }
        SketchEntity::Rectangle { corner_a, corner_b } => {
            *corner_a += delta;
            *corner_b += delta;
        }
        SketchEntity::Circle { center, .. } | SketchEntity::Arc { center, .. } => {
            *center += delta;
        }
    }
}

fn translate_vertex(sketch: &mut Sketch, point: EntityPoint, delta: DVec2) {
    let Some(entity) = sketch.entities.get_mut(point.entity()) else {
        return;
    };
    match (point, entity) {
        (EntityPoint::Point(_), SketchEntity::Point { p }) => *p += delta,
        (EntityPoint::Start(_), SketchEntity::Line { a, .. }) => *a += delta,
        (EntityPoint::End(_), SketchEntity::Line { b, .. }) => *b += delta,
        (EntityPoint::Center(_), SketchEntity::Circle { center, .. })
        | (EntityPoint::Center(_), SketchEntity::Arc { center, .. }) => {
            *center += delta;
        }
        // Arc endpoints and legacy rectangle corners are intentionally not
        // direct-manipulable in v1; selecting them still enables constraints.
        _ => {}
    }
}

fn add_constraint_once(sketch: &mut Sketch, constraint: Constraint) {
    if !sketch
        .iter_constraints()
        .any(|(_, existing)| *existing == constraint)
    {
        sketch.add_constraint(constraint);
    }
}

fn sketch_profile(profile: &ProfileRegion) -> SketchProfile {
    match profile {
        ProfileRegion::Polygon { points } => SketchProfile::Polygon {
            points: points.clone(),
        },
        ProfileRegion::Circle { center, radius } => SketchProfile::Circle {
            center: *center,
            radius: radius.as_f64(),
        },
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::constraint::{Constraint, EntityPoint};
    use roncad_core::selection::SelectionItem;
    use roncad_geometry::{Project, Sketch, SketchDimension};

    use super::apply;
    use roncad_core::command::AppCommand;
    use roncad_core::selection::Selection;
    use roncad_core::units::LengthMm;
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
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 3.0,
                });

        selection.insert(SelectionItem::SketchEntity { sketch, entity });
        apply(&mut project, &mut selection, &AppCommand::DeleteSelection);

        assert!(selection.is_empty());
        assert!(!project
            .active_sketch()
            .expect("active sketch")
            .entities
            .contains_key(entity));
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
    fn add_arc_persists_on_sketch() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");

        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddArc {
                sketch,
                center: dvec2(10.0, 10.0),
                radius: LengthMm::new(5.0),
                start_angle: 0.0,
                sweep_angle: std::f64::consts::FRAC_PI_2,
            },
        );

        let entities: Vec<_> = project
            .active_sketch()
            .expect("active sketch")
            .iter()
            .collect();
        assert!(entities.iter().any(|(_, entity)| {
            matches!(
                entity,
                SketchEntity::Arc {
                    center,
                    radius,
                    start_angle,
                    sweep_angle,
                }
                    if *center == dvec2(10.0, 10.0)
                        && (*radius - 5.0).abs() < f64::EPSILON
                        && (*start_angle - 0.0).abs() < f64::EPSILON
                        && (*sweep_angle - std::f64::consts::FRAC_PI_2).abs() < f64::EPSILON
            )
        }));
    }

    #[test]
    fn add_line_leaves_existing_crossings_intact() {
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

        assert_eq!(lines.len(), 2);
        assert!(contains_line(&lines, dvec2(0.0, 0.0), dvec2(10.0, 10.0)));
        assert!(contains_line(&lines, dvec2(0.0, 10.0), dvec2(10.0, 0.0)));
    }

    #[test]
    fn apply_line_fillet_replaces_selected_entities_with_trimmed_result() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let line_a = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });
        let line_b = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(0.0, 10.0),
            });
        selection.insert(SelectionItem::SketchEntity {
            sketch,
            entity: line_a,
        });
        selection.insert(SelectionItem::SketchEntity {
            sketch,
            entity: line_b,
        });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::ApplyLineFillet {
                sketch,
                line_a,
                line_b,
                corner: dvec2(0.0, 0.0),
                radius: LengthMm::new(2.0),
            },
        );

        let active_sketch = project.active_sketch().expect("active sketch");
        let selected_entities: Vec<_> = selection
            .iter()
            .filter_map(|item| match item {
                SelectionItem::SketchEntity {
                    sketch: selected_sketch,
                    entity,
                } if *selected_sketch == sketch => active_sketch.entities.get(*entity).cloned(),
                _ => None,
            })
            .collect();

        assert_eq!(selected_entities.len(), 3);
        assert!(selected_entities.iter().any(|entity| {
            matches!(
                entity,
                SketchEntity::Arc {
                    center,
                    radius,
                    sweep_angle,
                    ..
                }
                    if (*center - dvec2(2.0, 2.0)).length() < 1e-6
                        && (*radius - 2.0).abs() < 1e-6
                        && (sweep_angle.abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-6
            )
        }));

        let selected_lines: Vec<_> = selected_entities
            .iter()
            .filter_map(|entity| match entity {
                SketchEntity::Line { a, b } => Some((*a, *b)),
                _ => None,
            })
            .collect();
        assert_eq!(selected_lines.len(), 2);
        assert!(contains_line(
            &selected_lines,
            dvec2(10.0, 0.0),
            dvec2(2.0, 0.0)
        ));
        assert!(contains_line(
            &selected_lines,
            dvec2(0.0, 10.0),
            dvec2(0.0, 2.0)
        ));
    }

    #[test]
    fn set_line_length_preserves_direction_from_start() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(1.0, 2.0),
                b: dvec2(4.0, 6.0),
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::SetLineLength {
                sketch,
                entity,
                length: LengthMm::new(10.0),
            },
        );

        let stored = project
            .active_sketch()
            .expect("active sketch")
            .entities
            .get(entity)
            .cloned();
        let SketchEntity::Line { a, b } = stored.expect("line") else {
            panic!("expected line");
        };
        assert_eq!(a, dvec2(1.0, 2.0));
        assert!((a.distance(b) - 10.0).abs() < 1e-9);
        let dir = (b - a).normalize();
        assert!((dir - dvec2(0.6, 0.8)).length() < 1e-9);
    }

    #[test]
    fn set_rectangle_width_preserves_anchor_and_height() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Rectangle {
                    corner_a: dvec2(2.0, 3.0),
                    corner_b: dvec2(7.0, 9.0),
                });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::SetRectangleWidth {
                sketch,
                entity,
                width: LengthMm::new(20.0),
            },
        );

        let SketchEntity::Rectangle { corner_a, corner_b } = project
            .active_sketch()
            .expect("active sketch")
            .entities
            .get(entity)
            .expect("rect")
            .clone()
        else {
            panic!("expected rectangle");
        };
        assert_eq!(corner_a, dvec2(2.0, 3.0));
        assert_eq!(corner_b, dvec2(22.0, 9.0));
    }

    #[test]
    fn add_rectangle_normalizes_to_lines_and_constraints() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");

        apply(
            &mut project,
            &mut selection,
            &AppCommand::AddRectangle {
                sketch,
                corner_a: dvec2(1.0, 2.0),
                corner_b: dvec2(5.0, 6.0),
            },
        );

        let sketch_ref = project.active_sketch().expect("active sketch");
        let lines: Vec<_> = sketch_ref
            .iter()
            .filter_map(|(_, entity)| match entity {
                SketchEntity::Line { a, b } => Some((*a, *b)),
                SketchEntity::Rectangle { .. } => panic!("new rectangles should be canonical"),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 4);
        assert!(lines.contains(&(dvec2(1.0, 2.0), dvec2(5.0, 2.0))));
        assert!(lines.contains(&(dvec2(5.0, 2.0), dvec2(5.0, 6.0))));
        assert!(lines.contains(&(dvec2(5.0, 6.0), dvec2(1.0, 6.0))));
        assert!(lines.contains(&(dvec2(1.0, 6.0), dvec2(1.0, 2.0))));

        let horizontal = sketch_ref
            .iter_constraints()
            .filter(|(_, c)| matches!(c, Constraint::Horizontal { .. }))
            .count();
        let vertical = sketch_ref
            .iter_constraints()
            .filter(|(_, c)| matches!(c, Constraint::Vertical { .. }))
            .count();
        let coincident = sketch_ref
            .iter_constraints()
            .filter(|(_, c)| matches!(c, Constraint::Coincident { .. }))
            .count();
        assert_eq!(horizontal, 2);
        assert_eq!(vertical, 2);
        assert_eq!(coincident, 4);
    }

    #[test]
    fn translate_selection_moves_one_line_endpoint() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let line = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![EntityPoint::Start(line)],
                entities: vec![],
                delta: dvec2(2.0, 3.0),
            },
        );

        let SketchEntity::Line { a, b } = project.active_sketch().unwrap().entities[line] else {
            panic!("expected line");
        };
        assert_eq!(a, dvec2(2.0, 3.0));
        assert_eq!(b, dvec2(10.0, 0.0));
    }

    #[test]
    fn translate_selection_moves_full_line_entity() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let line = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![],
                entities: vec![line],
                delta: dvec2(2.0, 3.0),
            },
        );

        let SketchEntity::Line { a, b } = project.active_sketch().unwrap().entities[line] else {
            panic!("expected line");
        };
        assert_eq!(a, dvec2(2.0, 3.0));
        assert_eq!(b, dvec2(12.0, 3.0));
    }

    #[test]
    fn translate_selection_moves_vertices_from_different_lines() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let first = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(1.0, 0.0),
            });
        let second = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(5.0, 0.0),
                b: dvec2(6.0, 0.0),
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![EntityPoint::End(first), EntityPoint::Start(second)],
                entities: vec![],
                delta: dvec2(0.0, 2.0),
            },
        );

        let SketchEntity::Line { b: first_b, .. } =
            project.active_sketch().unwrap().entities[first]
        else {
            panic!("expected line");
        };
        let SketchEntity::Line { a: second_a, .. } =
            project.active_sketch().unwrap().entities[second]
        else {
            panic!("expected line");
        };
        assert_eq!(first_b, dvec2(1.0, 2.0));
        assert_eq!(second_a, dvec2(5.0, 2.0));
    }

    #[test]
    fn translate_selection_dedupes_entity_and_vertex() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let line = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![EntityPoint::Start(line)],
                entities: vec![line],
                delta: dvec2(2.0, 0.0),
            },
        );

        let SketchEntity::Line { a, b } = project.active_sketch().unwrap().entities[line] else {
            panic!("expected line");
        };
        assert_eq!(a, dvec2(2.0, 0.0));
        assert_eq!(b, dvec2(12.0, 0.0));
    }

    #[test]
    fn translate_fixed_point_runs_solver_without_corrupting_constraint() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let point = project
            .active_sketch_mut()
            .unwrap()
            .add(SketchEntity::Point { p: dvec2(1.0, 1.0) });
        project
            .active_sketch_mut()
            .unwrap()
            .add_constraint(Constraint::FixPoint {
                point: EntityPoint::Point(point),
                target: dvec2(1.0, 1.0),
            });

        let report = apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![EntityPoint::Point(point)],
                entities: vec![],
                delta: dvec2(5.0, 0.0),
            },
        );

        assert!(report.is_some());
        let SketchEntity::Point { p } = project.active_sketch().unwrap().entities[point] else {
            panic!("expected point");
        };
        assert!((p - dvec2(1.0, 1.0)).length() < 1e-6);
        assert_eq!(
            project.active_sketch().unwrap().iter_constraints().count(),
            1
        );
    }

    #[test]
    fn translate_arc_endpoint_is_ignored() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let arc = project.active_sketch_mut().unwrap().add(SketchEntity::Arc {
            center: dvec2(0.0, 0.0),
            radius: 5.0,
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::TranslateSketchSelection {
                sketch,
                vertices: vec![EntityPoint::Start(arc)],
                entities: vec![],
                delta: dvec2(2.0, 0.0),
            },
        );

        let SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } = project.active_sketch().unwrap().entities[arc]
        else {
            panic!("expected arc");
        };
        assert_eq!(center, dvec2(0.0, 0.0));
        assert_eq!(radius, 5.0);
        assert_eq!(start_angle, 0.0);
        assert_eq!(sweep_angle, std::f64::consts::FRAC_PI_2);
    }

    #[test]
    fn set_arc_sweep_converts_degrees_to_radians() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Arc {
                center: dvec2(0.0, 0.0),
                radius: 4.0,
                start_angle: 0.0,
                sweep_angle: std::f64::consts::FRAC_PI_2,
            });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::SetArcSweepDegrees {
                sketch,
                entity,
                sweep_degrees: 45.0,
            },
        );

        let SketchEntity::Arc { sweep_angle, .. } = project
            .active_sketch()
            .expect("active sketch")
            .entities
            .get(entity)
            .expect("arc")
            .clone()
        else {
            panic!("expected arc");
        };
        assert!((sweep_angle - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
    }

    #[test]
    fn add_constraint_skips_duplicates() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 2.0),
            });

        let command = AppCommand::AddConstraint {
            sketch,
            constraint: Constraint::Horizontal { entity },
        };
        apply(&mut project, &mut selection, &command);
        apply(&mut project, &mut selection, &command);

        assert_eq!(
            project
                .active_sketch()
                .expect("active sketch")
                .constraints
                .len(),
            1
        );
    }

    #[test]
    fn remove_constraint_deletes_existing_constraint() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 0.0),
            });
        let constraint_id = project
            .active_sketch_mut()
            .expect("active sketch")
            .add_constraint(Constraint::Horizontal { entity });

        apply(
            &mut project,
            &mut selection,
            &AppCommand::RemoveConstraint {
                sketch,
                constraint: constraint_id,
            },
        );

        assert!(project
            .active_sketch()
            .expect("active sketch")
            .iter_constraints()
            .next()
            .is_none());
    }

    #[test]
    fn mutating_linked_sketch_rebuilds_body_feature() {
        let mut project = Project::new_untitled();
        let mut selection = Selection::default();
        let sketch = project.active_sketch.expect("default project has sketch");
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 2.0,
                });
        let (body, feature) = project
            .extrude_profile(
                sketch,
                roncad_geometry::SketchProfile::Circle {
                    center: dvec2(5.0, 5.0),
                    radius: 2.0,
                },
                5.0,
            )
            .expect("extrude");

        apply(
            &mut project,
            &mut selection,
            &AppCommand::SetCircleRadius {
                sketch,
                entity,
                radius: LengthMm::new(4.0),
            },
        );

        assert!(project.features[feature].is_profile_valid());
        assert_eq!(
            project.body_volume_mm3(body),
            std::f64::consts::PI * 16.0 * 5.0
        );
    }

    fn contains_line(lines: &[(glam::DVec2, glam::DVec2)], a: glam::DVec2, b: glam::DVec2) -> bool {
        lines.iter().any(|(line_a, line_b)| {
            (same_point(*line_a, a) && same_point(*line_b, b))
                || (same_point(*line_a, b) && same_point(*line_b, a))
        })
    }

    fn same_point(a: glam::DVec2, b: glam::DVec2) -> bool {
        a.distance_squared(b) < 1e-9
    }
}
