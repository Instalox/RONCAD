//! Shared read-only dimension descriptions for inspector and viewport overlays.
//! Keeps sketch measurement formatting in one place inside the UI layer.

use egui::{Align2, Color32, Vec2};
use glam::DVec2;
use roncad_core::constraint::EntityPoint;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{
    arc_end_point, arc_mid_point, arc_start_point, resolve_entity_point, HoverTarget, Project,
    SketchDimension, SketchEntity,
};
use slotmap::Key;

use crate::theme::ThemeColors;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DimensionValue {
    pub label: &'static str,
    pub value_mm: f64,
}

impl DimensionValue {
    pub fn formatted_value(&self) -> String {
        format_mm(self.value_mm)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DimensionAnnotation {
    pub anchor_world: DVec2,
    pub span_world: Option<(DVec2, DVec2)>,
    pub offset_px: Vec2,
    pub align: Align2,
    pub text: String,
    pub color: Color32,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityDimensions {
    pub kind: &'static str,
    pub tag: String,
    pub summary: Vec<DimensionValue>,
    pub annotations: Vec<DimensionAnnotation>,
}

pub(crate) fn selected_entity_dimensions(
    project: &Project,
    selection: &Selection,
) -> Vec<EntityDimensions> {
    let Some(sketch_id) = project.active_sketch else {
        return Vec::new();
    };
    let Some(sketch) = project.active_sketch() else {
        return Vec::new();
    };

    let mut dimensions = Vec::new();
    for (entity_id, entity) in sketch.iter() {
        if selection.contains(&SelectionItem::SketchEntity {
            sketch: sketch_id,
            entity: entity_id,
        }) {
            dimensions.push(describe_entity(entity_id, entity));
        }
    }
    dimensions
}

pub(crate) fn active_sketch_dimension_annotations(project: &Project) -> Vec<DimensionAnnotation> {
    let Some(sketch) = project.active_sketch() else {
        return Vec::new();
    };

    sketch
        .iter_dimensions()
        .map(|(_, dimension)| describe_sketch_dimension_annotation(dimension))
        .collect()
}

pub(crate) fn hovered_target_summary(
    project: &Project,
    hovered: Option<&HoverTarget>,
) -> Option<String> {
    match hovered? {
        HoverTarget::SketchEntity { sketch, entity } => {
            let sketch = project.sketches.get(*sketch)?;
            let entity = sketch.entities.get(*entity)?;

            Some(match entity {
                SketchEntity::Point { p } => {
                    format!(
                        "Hover Point   X {}   Y {}",
                        format_value(p.x),
                        format_value(p.y)
                    )
                }
                SketchEntity::Line { a, b } => {
                    format!("Hover Line   L {}", format_mm(a.distance(*b)))
                }
                SketchEntity::Rectangle { corner_a, corner_b } => {
                    let min = corner_a.min(*corner_b);
                    let max = corner_a.max(*corner_b);
                    format!(
                        "Hover Rectangle   W {}   H {}",
                        format_mm((max.x - min.x).abs()),
                        format_mm((max.y - min.y).abs()),
                    )
                }
                SketchEntity::Circle { radius, .. } => {
                    format!("Hover Circle   R {}", format_mm(*radius))
                }
                SketchEntity::Arc {
                    radius,
                    sweep_angle,
                    ..
                } => {
                    format!(
                        "Hover Arc   R {}   A {:.1} deg",
                        format_mm(*radius),
                        sweep_angle.to_degrees().abs(),
                    )
                }
            })
        }
        HoverTarget::SketchVertex { sketch, point } => {
            let sketch = project.sketches.get(*sketch)?;
            let entity = sketch.entities.get(point.entity())?;
            let p = resolve_entity_point(*point, entity)?;
            Some(format!(
                "Hover {}   X {}   Y {}",
                entity_point_label(*point),
                format_value(p.x),
                format_value(p.y)
            ))
        }
        HoverTarget::Profile { profile, .. } => {
            Some(format!("Hover Profile   A {:.3} mm^2", profile.area()))
        }
    }
}

fn entity_point_label(point: EntityPoint) -> &'static str {
    match point {
        EntityPoint::Point(_) => "Point",
        EntityPoint::Start(_) | EntityPoint::End(_) => "Endpoint",
        EntityPoint::Center(_) => "Center",
    }
}

fn describe_entity(
    entity_id: roncad_core::ids::SketchEntityId,
    entity: &SketchEntity,
) -> EntityDimensions {
    let tag = entity_tag(entity_id, entity);
    match entity {
        SketchEntity::Point { p } => EntityDimensions {
            kind: entity.kind_name(),
            tag,
            summary: vec![
                DimensionValue {
                    label: "X",
                    value_mm: p.x,
                },
                DimensionValue {
                    label: "Y",
                    value_mm: p.y,
                },
            ],
            annotations: vec![DimensionAnnotation {
                anchor_world: *p,
                span_world: None,
                offset_px: Vec2::new(10.0, -10.0),
                align: Align2::LEFT_BOTTOM,
                text: format!("X {}\nY {}", format_mm(p.x), format_mm(p.y)),
                color: ThemeColors::TEXT,
            }],
        },
        SketchEntity::Line { a, b } => {
            let length = a.distance(*b);
            let midpoint = (*a + *b) * 0.5;
            EntityDimensions {
                kind: entity.kind_name(),
                tag,
                summary: vec![DimensionValue {
                    label: "Length",
                    value_mm: length,
                }],
                annotations: vec![DimensionAnnotation {
                    anchor_world: midpoint,
                    span_world: None,
                    offset_px: Vec2::new(0.0, -10.0),
                    align: Align2::CENTER_BOTTOM,
                    text: format!("L {}", format_mm(length)),
                    color: ThemeColors::TEXT,
                }],
            }
        }
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let min = corner_a.min(*corner_b);
            let max = corner_a.max(*corner_b);
            let width = (max.x - min.x).abs();
            let height = (max.y - min.y).abs();
            EntityDimensions {
                kind: entity.kind_name(),
                tag,
                summary: vec![
                    DimensionValue {
                        label: "Width",
                        value_mm: width,
                    },
                    DimensionValue {
                        label: "Height",
                        value_mm: height,
                    },
                ],
                annotations: vec![
                    DimensionAnnotation {
                        anchor_world: DVec2::new((min.x + max.x) * 0.5, max.y),
                        span_world: None,
                        offset_px: Vec2::new(0.0, -10.0),
                        align: Align2::CENTER_BOTTOM,
                        text: format!("W {}", format_mm(width)),
                        color: ThemeColors::TEXT,
                    },
                    DimensionAnnotation {
                        anchor_world: DVec2::new(max.x, (min.y + max.y) * 0.5),
                        span_world: None,
                        offset_px: Vec2::new(10.0, 0.0),
                        align: Align2::LEFT_CENTER,
                        text: format!("H {}", format_mm(height)),
                        color: ThemeColors::TEXT,
                    },
                ],
            }
        }
        SketchEntity::Circle { center, radius } => EntityDimensions {
            kind: entity.kind_name(),
            tag,
            summary: vec![
                DimensionValue {
                    label: "Radius",
                    value_mm: *radius,
                },
                DimensionValue {
                    label: "Diameter",
                    value_mm: *radius * 2.0,
                },
            ],
            annotations: vec![DimensionAnnotation {
                anchor_world: *center + DVec2::new(*radius, 0.0),
                span_world: None,
                offset_px: Vec2::new(10.0, 0.0),
                align: Align2::LEFT_CENTER,
                text: format!("R {}", format_mm(*radius)),
                color: ThemeColors::TEXT,
            }],
        },
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => {
            let midpoint = arc_mid_point(*center, *radius, *start_angle, *sweep_angle);
            let start = arc_start_point(*center, *radius, *start_angle);
            let end = arc_end_point(*center, *radius, *start_angle, *sweep_angle);
            EntityDimensions {
                kind: entity.kind_name(),
                tag,
                summary: vec![
                    DimensionValue {
                        label: "Radius",
                        value_mm: *radius,
                    },
                    DimensionValue {
                        label: "Arc Length",
                        value_mm: radius * sweep_angle.abs(),
                    },
                    DimensionValue {
                        label: "Sweep",
                        value_mm: sweep_angle.to_degrees().abs(),
                    },
                ],
                annotations: vec![
                    DimensionAnnotation {
                        anchor_world: midpoint,
                        span_world: None,
                        offset_px: Vec2::new(0.0, -10.0),
                        align: Align2::CENTER_BOTTOM,
                        text: format!("R {}", format_mm(*radius)),
                        color: ThemeColors::TEXT,
                    },
                    DimensionAnnotation {
                        anchor_world: (start + end) * 0.5,
                        span_world: None,
                        offset_px: Vec2::new(10.0, 0.0),
                        align: Align2::LEFT_CENTER,
                        text: format!("{:.1} deg", sweep_angle.to_degrees().abs()),
                        color: ThemeColors::TEXT,
                    },
                ],
            }
        }
    }
}

fn entity_tag(entity_id: roncad_core::ids::SketchEntityId, entity: &SketchEntity) -> String {
    let prefix = match entity {
        SketchEntity::Point { .. } => "p",
        SketchEntity::Line { .. } => "l",
        SketchEntity::Rectangle { .. } => "r",
        SketchEntity::Circle { .. } => "c",
        SketchEntity::Arc { .. } => "a",
    };
    let slot = (entity_id.data().as_ffi() & 0xffff_ffff) as u32;
    format!("{prefix}_{:03}", slot)
}

fn describe_sketch_dimension_annotation(dimension: &SketchDimension) -> DimensionAnnotation {
    match dimension {
        SketchDimension::Distance { start, end } => {
            let delta = *end - *start;
            let (offset_px, align) = dimension_label_offset(delta);
            DimensionAnnotation {
                anchor_world: (*start + *end) * 0.5,
                span_world: Some((*start, *end)),
                offset_px,
                align,
                text: format_value(start.distance(*end)),
                color: ThemeColors::ACCENT_AMBER,
            }
        }
    }
}

fn dimension_label_offset(delta: DVec2) -> (Vec2, Align2) {
    if delta.x.abs() >= delta.y.abs() * 1.5 {
        (Vec2::new(0.0, -14.0), Align2::CENTER_BOTTOM)
    } else if delta.y.abs() >= delta.x.abs() * 1.5 {
        (Vec2::new(12.0, 0.0), Align2::LEFT_CENTER)
    } else {
        (Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM)
    }
}

fn format_mm(value_mm: f64) -> String {
    format!("{value_mm:.3} mm")
}

fn format_value(value_mm: f64) -> String {
    format!("{value_mm:.3}")
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::selection::{Selection, SelectionItem};
    use roncad_geometry::{HoverTarget, Project, SketchDimension, SketchEntity};

    use super::{
        active_sketch_dimension_annotations, hovered_target_summary, selected_entity_dimensions,
    };
    use crate::theme::ThemeColors;

    #[test]
    fn selected_line_reports_length() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let entity = project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Line {
                a: dvec2(0.0, 0.0),
                b: dvec2(3.0, 4.0),
            });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity { sketch, entity });

        let dims = selected_entity_dimensions(&project, &selection);

        assert_eq!(dims.len(), 1);
        assert_eq!(dims[0].kind, "Line");
        assert_eq!(dims[0].summary[0].label, "Length");
        assert_eq!(dims[0].summary[0].value_mm, 5.0);
        assert_eq!(dims[0].annotations[0].text, "L 5.000 mm");
    }

    #[test]
    fn selected_rectangle_reports_width_and_height() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Rectangle {
                    corner_a: dvec2(2.0, 3.0),
                    corner_b: dvec2(12.0, 9.0),
                });
        let mut selection = Selection::default();
        selection.insert(SelectionItem::SketchEntity { sketch, entity });

        let dims = selected_entity_dimensions(&project, &selection);

        assert_eq!(dims.len(), 1);
        assert_eq!(dims[0].summary.len(), 2);
        assert_eq!(dims[0].summary[0].label, "Width");
        assert_eq!(dims[0].summary[0].value_mm, 10.0);
        assert_eq!(dims[0].summary[1].label, "Height");
        assert_eq!(dims[0].summary[1].value_mm, 6.0);
        assert_eq!(dims[0].annotations[0].text, "W 10.000 mm");
        assert_eq!(dims[0].annotations[1].text, "H 6.000 mm");
    }

    #[test]
    fn sketch_distance_dimension_annotation_is_amber_value_only() {
        let mut project = Project::new_untitled();
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add_dimension(SketchDimension::Distance {
                start: dvec2(0.0, 0.0),
                end: dvec2(0.0, 7.5),
            });

        let annotations = active_sketch_dimension_annotations(&project);

        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].text, "7.500");
        assert_eq!(annotations[0].color, ThemeColors::ACCENT_AMBER);
    }

    #[test]
    fn hovered_rectangle_reports_compact_summary() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let entity =
            project
                .active_sketch_mut()
                .expect("active sketch")
                .add(SketchEntity::Rectangle {
                    corner_a: dvec2(2.0, 3.0),
                    corner_b: dvec2(12.0, 9.0),
                });

        let summary =
            hovered_target_summary(&project, Some(&HoverTarget::sketch_entity(sketch, entity)));

        assert_eq!(
            summary.as_deref(),
            Some("Hover Rectangle   W 10.000 mm   H 6.000 mm")
        );
    }

    #[test]
    fn hovered_profile_reports_area_summary() {
        let mut project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        project
            .active_sketch_mut()
            .expect("active sketch")
            .add(SketchEntity::Circle {
                center: dvec2(10.0, 10.0),
                radius: 4.0,
            });
        let profile = roncad_geometry::pick_closed_profile(
            project.active_sketch().expect("active sketch"),
            dvec2(10.0, 10.0),
        )
        .expect("profile");

        let summary =
            hovered_target_summary(&project, Some(&HoverTarget::profile(sketch, profile)));

        assert_eq!(summary.as_deref(), Some("Hover Profile   A 50.265 mm^2"));
    }
}
