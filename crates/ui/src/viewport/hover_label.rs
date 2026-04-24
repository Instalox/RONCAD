//! Cursor-adjacent hover label. When Select is active and the cursor sits
//! over a sketch entity or profile, a compact pill appears just below-right
//! of the cursor ("Line · 6.3 mm"). Keeps the readout where the eye already
//! is instead of forcing a trip to the top-left HUD.

use egui::{Area, Frame, Id, Label, Margin, Order, Rect, RichText, Stroke, TextWrapMode, Ui};
use roncad_geometry::{HoverTarget, Project, SketchEntity};
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
    let Some(label) = compact_label(shell.project, hovered) else {
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

pub(crate) fn compact_label(project: &Project, hovered: &HoverTarget) -> Option<String> {
    match hovered {
        HoverTarget::SketchEntity { sketch, entity } => {
            let sketch = project.sketches.get(*sketch)?;
            let entity = sketch.entities.get(*entity)?;
            Some(entity_label(entity))
        }
        HoverTarget::Profile { profile, .. } => {
            Some(format!("Profile · {} mm²", trim_mm(profile.area())))
        }
    }
}

fn entity_label(entity: &SketchEntity) -> String {
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
        assert_eq!(compact_label(&project, &target).as_deref(), Some("Line · 6 mm"));
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
        assert_eq!(compact_label(&project, &target).as_deref(), Some("Circle · R 2.5 mm"));
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
        assert_eq!(compact_label(&project, &target).as_deref(), Some("Arc · R 3 mm · 90°"));
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
        assert_eq!(compact_label(&project, &target).as_deref(), Some("Rect · 10 × 4.5 mm"));
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
