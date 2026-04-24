//! Cursor-adjacent hover label. When Select is active and the cursor sits
//! over a sketch entity or profile, a compact pill appears just below-right
//! of the cursor ("Line · 6.3 mm"). Keeps the readout where the eye already
//! is instead of forcing a trip to the top-left HUD.

use egui::{Area, Frame, Id, Label, Margin, Order, Rect, RichText, Stroke, TextWrapMode, Ui};
use roncad_core::constraint::EntityPoint;
use roncad_geometry::{resolve_entity_point, HoverTarget, Project, SketchEntity};
use roncad_tools::ActiveToolKind;

use crate::shell::ShellContext;
use crate::theme::ThemeColors;

const CURSOR_OFFSET: egui::Vec2 = egui::vec2(14.0, 18.0);

pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    shell: &ShellContext<'_>,
    hovered: Option<&HoverTarget>,
) {
    if shell.tool_manager.active_kind() != ActiveToolKind::Select {
        return;
    }
    let Some(hovered) = hovered else {
        return;
    };
    let Some(label) = compact_label_with_cursor(
        shell.project,
        hovered,
        *shell.cursor_world_mm,
        shell.camera.pixels_per_mm,
    ) else {
        return;
    };
    let Some(cursor) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    if !rect.contains(cursor) {
        return;
    }

    let anchor = cursor + CURSOR_OFFSET;
    Area::new(Id::new("viewport_hover_label"))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(ThemeColors::BG_PANEL_GLASS)
                .stroke(Stroke::new(1.0, ThemeColors::SEPARATOR_SOFT))
                .inner_margin(Margin::symmetric(7, 3))
                .corner_radius(4.0_f32)
                .show(ui, |ui| {
                    ui.add(
                        Label::new(
                            RichText::new(label)
                                .monospace()
                                .size(10.5)
                                .color(ThemeColors::TEXT),
                        )
                        .wrap_mode(TextWrapMode::Extend),
                    );
                });
        });
}

#[cfg(test)]
pub(crate) fn compact_label(project: &Project, hovered: &HoverTarget) -> Option<String> {
    compact_label_with_cursor(project, hovered, None, 1.0)
}

fn compact_label_with_cursor(
    project: &Project,
    hovered: &HoverTarget,
    cursor_world_mm: Option<glam::DVec2>,
    pixels_per_mm: f64,
) -> Option<String> {
    match hovered {
        HoverTarget::SketchEntity { sketch, entity } => {
            let sketch = project.sketches.get(*sketch)?;
            let entity = sketch.entities.get(*entity)?;
            Some(entity_label(entity, cursor_world_mm, pixels_per_mm))
        }
        HoverTarget::SketchVertex { sketch, point } => {
            let sketch = project.sketches.get(*sketch)?;
            let entity = sketch.entities.get(point.entity())?;
            let position = resolve_entity_point(*point, entity)?;
            Some(format!(
                "{} · ({}, {})",
                point_label(*point),
                trim_mm(position.x),
                trim_mm(position.y)
            ))
        }
        HoverTarget::Profile { profile, .. } => {
            Some(format!("Profile · {} mm²", trim_mm(profile.area())))
        }
    }
}

fn point_label(point: EntityPoint) -> &'static str {
    match point {
        EntityPoint::Point(_) => "Point",
        EntityPoint::Start(_) | EntityPoint::End(_) => "Endpoint",
        EntityPoint::Center(_) => "Center",
        EntityPoint::CornerA(_)
        | EntityPoint::CornerB(_)
        | EntityPoint::CornerC(_)
        | EntityPoint::CornerD(_) => "Corner",
    }
}

fn entity_label(
    entity: &SketchEntity,
    cursor_world_mm: Option<glam::DVec2>,
    pixels_per_mm: f64,
) -> String {
    if let Some(label) = endpoint_label(entity, cursor_world_mm, pixels_per_mm) {
        return label;
    }
    match entity {
        SketchEntity::Point { p } => {
            format!("Point · ({}, {})", trim_mm(p.x), trim_mm(p.y))
        }
        SketchEntity::Line { a, b } => {
            format!("Line · {} mm", trim_mm(a.distance(*b)))
        }
        SketchEntity::Rectangle { corner_a, corner_b } => {
            let w = (corner_b.x - corner_a.x).abs();
            let h = (corner_b.y - corner_a.y).abs();
            format!("Rect · {} × {} mm", trim_mm(w), trim_mm(h))
        }
        SketchEntity::Circle { radius, .. } => {
            format!("Circle · R {} mm", trim_mm(*radius))
        }
        SketchEntity::Arc {
            radius,
            sweep_angle,
            ..
        } => {
            format!(
                "Arc · R {} mm · {}°",
                trim_mm(*radius),
                trim_angle(sweep_angle.to_degrees().abs()),
            )
        }
    }
}

fn endpoint_label(
    entity: &SketchEntity,
    cursor_world_mm: Option<glam::DVec2>,
    pixels_per_mm: f64,
) -> Option<String> {
    let cursor = cursor_world_mm?;
    let tolerance = 9.0 / pixels_per_mm.max(f64::EPSILON);
    let mut candidates: Vec<(&'static str, glam::DVec2)> = match entity {
        SketchEntity::Point { p } => vec![("Point", *p)],
        SketchEntity::Line { a, b } => vec![("Endpoint", *a), ("Endpoint", *b)],
        SketchEntity::Rectangle { corner_a, corner_b } => vec![
            ("Corner", *corner_a),
            ("Corner", glam::dvec2(corner_b.x, corner_a.y)),
            ("Corner", *corner_b),
            ("Corner", glam::dvec2(corner_a.x, corner_b.y)),
        ],
        SketchEntity::Circle { center, .. } => vec![("Center", *center)],
        SketchEntity::Arc {
            center,
            radius,
            start_angle,
            sweep_angle,
        } => vec![
            (
                "Endpoint",
                *center + glam::DVec2::new(start_angle.cos(), start_angle.sin()) * *radius,
            ),
            (
                "Endpoint",
                *center
                    + glam::DVec2::new(
                        (*start_angle + *sweep_angle).cos(),
                        (*start_angle + *sweep_angle).sin(),
                    ) * *radius,
            ),
            ("Center", *center),
        ],
    };
    candidates.sort_by(|(_, a), (_, b)| {
        a.distance_squared(cursor)
            .partial_cmp(&b.distance_squared(cursor))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let (kind, point) = candidates.first().copied()?;
    (point.distance(cursor) <= tolerance)
        .then(|| format!("{} · ({}, {})", kind, trim_mm(point.x), trim_mm(point.y)))
}

fn trim_mm(value: f64) -> String {
    trim_trailing_zeros(&format!("{value:.2}"))
}

fn trim_angle(value_deg: f64) -> String {
    trim_trailing_zeros(&format!("{value_deg:.1}"))
}

fn trim_trailing_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::dvec2;
    use roncad_geometry::Project;

    fn sketch_entity_target(project: &Project, entity: &SketchEntity) -> HoverTarget {
        let _ = entity; // silence unused in some branches
        let sketch = project.active_sketch.unwrap();
        // Re-lookup the entity id: the caller will have just added it.
        let entity_id = project.active_sketch().unwrap().iter().last().unwrap().0;
        HoverTarget::sketch_entity(sketch, entity_id)
    }

    #[test]
    fn line_label_uses_compact_format() {
        let mut project = Project::new_untitled();
        let line = SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(6.0, 0.0),
        };
        project.active_sketch_mut().unwrap().add(line.clone());
        let target = sketch_entity_target(&project, &line);
        assert_eq!(
            compact_label(&project, &target).as_deref(),
            Some("Line · 6 mm")
        );
    }

    #[test]
    fn circle_label_shows_radius() {
        let mut project = Project::new_untitled();
        let circle = SketchEntity::Circle {
            center: dvec2(0.0, 0.0),
            radius: 2.5,
        };
        project.active_sketch_mut().unwrap().add(circle.clone());
        let target = sketch_entity_target(&project, &circle);
        assert_eq!(
            compact_label(&project, &target).as_deref(),
            Some("Circle · R 2.5 mm")
        );
    }

    #[test]
    fn arc_label_shows_radius_and_sweep() {
        let mut project = Project::new_untitled();
        let arc = SketchEntity::Arc {
            center: dvec2(0.0, 0.0),
            radius: 3.0,
            start_angle: 0.0,
            sweep_angle: std::f64::consts::FRAC_PI_2,
        };
        project.active_sketch_mut().unwrap().add(arc.clone());
        let target = sketch_entity_target(&project, &arc);
        assert_eq!(
            compact_label(&project, &target).as_deref(),
            Some("Arc · R 3 mm · 90°")
        );
    }

    #[test]
    fn rectangle_label_shows_size() {
        let mut project = Project::new_untitled();
        let rect = SketchEntity::Rectangle {
            corner_a: dvec2(0.0, 0.0),
            corner_b: dvec2(10.0, 4.5),
        };
        project.active_sketch_mut().unwrap().add(rect.clone());
        let target = sketch_entity_target(&project, &rect);
        assert_eq!(
            compact_label(&project, &target).as_deref(),
            Some("Rect · 10 × 4.5 mm")
        );
    }

    #[test]
    fn trim_trailing_zeros_keeps_significant_digits() {
        assert_eq!(trim_trailing_zeros("6.00"), "6");
        assert_eq!(trim_trailing_zeros("6.30"), "6.3");
        assert_eq!(trim_trailing_zeros("6.35"), "6.35");
        assert_eq!(trim_trailing_zeros("0.10"), "0.1");
        assert_eq!(trim_trailing_zeros("100"), "100");
    }
}
