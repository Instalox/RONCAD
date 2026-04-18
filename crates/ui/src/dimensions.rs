//! Shared read-only dimension descriptions for inspector and viewport overlays.
//! Keeps sketch measurement formatting in one place inside the UI layer.

use egui::{Align2, Vec2};
use glam::DVec2;
use roncad_core::selection::{Selection, SelectionItem};
use roncad_geometry::{Project, SketchEntity};

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
    pub offset_px: Vec2,
    pub align: Align2,
    pub text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityDimensions {
    pub kind: &'static str,
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
            dimensions.push(describe_entity(entity));
        }
    }
    dimensions
}

fn describe_entity(entity: &SketchEntity) -> EntityDimensions {
    match entity {
        SketchEntity::Point { p } => EntityDimensions {
            kind: entity.kind_name(),
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
                offset_px: Vec2::new(10.0, -10.0),
                align: Align2::LEFT_BOTTOM,
                text: format!("X {}\nY {}", format_mm(p.x), format_mm(p.y)),
            }],
        },
        SketchEntity::Line { a, b } => {
            let length = a.distance(*b);
            let midpoint = (*a + *b) * 0.5;
            EntityDimensions {
                kind: entity.kind_name(),
                summary: vec![DimensionValue {
                    label: "Length",
                    value_mm: length,
                }],
                annotations: vec![DimensionAnnotation {
                    anchor_world: midpoint,
                    offset_px: Vec2::new(0.0, -10.0),
                    align: Align2::CENTER_BOTTOM,
                    text: format!("L {}", format_mm(length)),
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
                        offset_px: Vec2::new(0.0, -10.0),
                        align: Align2::CENTER_BOTTOM,
                        text: format!("W {}", format_mm(width)),
                    },
                    DimensionAnnotation {
                        anchor_world: DVec2::new(max.x, (min.y + max.y) * 0.5),
                        offset_px: Vec2::new(10.0, 0.0),
                        align: Align2::LEFT_CENTER,
                        text: format!("H {}", format_mm(height)),
                    },
                ],
            }
        }
        SketchEntity::Circle { center, radius } => EntityDimensions {
            kind: entity.kind_name(),
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
                offset_px: Vec2::new(10.0, 0.0),
                align: Align2::LEFT_CENTER,
                text: format!("R {}", format_mm(*radius)),
            }],
        },
    }
}

fn format_mm(value_mm: f64) -> String {
    format!("{value_mm:.3} mm")
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::selection::{Selection, SelectionItem};
    use roncad_geometry::{Project, SketchEntity};

    use super::selected_entity_dimensions;

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
        let entity = project
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
}
