//! Circle tool: click the center, click a point on the rim.

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::units::LengthMm;

use crate::tool::{ActiveToolKind, DynamicField, Tool, ToolContext, ToolPreview, DYN_FIELD_RADIUS};

const CIRCLE_DYN_FIELDS: &[DynamicField] = &[DYN_FIELD_RADIUS];

#[derive(Default)]
pub struct CircleTool {
    center: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for CircleTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Circle
    }

    fn on_pointer_move(&mut self, _ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(world_mm);
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };

        match self.center.take() {
            None => {
                self.center = Some(world_mm);
                self.cursor = Some(world_mm);
                Vec::new()
            }
            Some(center) => {
                let radius = center.distance(world_mm);
                if radius < f64::EPSILON {
                    self.center = Some(world_mm);
                    return Vec::new();
                }
                vec![AppCommand::AddCircle {
                    sketch,
                    center,
                    radius: LengthMm::new(radius),
                }]
            }
        }
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.center = None;
        self.cursor = None;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.center = None;
        self.cursor = None;
    }

    fn preview(&self) -> ToolPreview {
        match (self.center, self.cursor) {
            (Some(c), Some(p)) => ToolPreview::Circle {
                center: c,
                radius: c.distance(p),
            },
            _ => ToolPreview::None,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.center {
            None => "Click center point. Shortcut: C. Right-click or Esc cancels.".to_string(),
            Some(_) => "Type radius and press Enter — or click rim. Right-click or Esc cancels."
                .to_string(),
        })
    }

    fn dynamic_fields(&self) -> &'static [DynamicField] {
        if self.center.is_some() {
            CIRCLE_DYN_FIELDS
        } else {
            &[]
        }
    }

    fn dynamic_preview(&self, values: &[Option<f64>]) -> Option<ToolPreview> {
        let (center, radius) = resolved_circle(self.center?, self.cursor?, values)?;
        Some(ToolPreview::Circle { center, radius })
    }

    fn dynamic_display_values(&self, values: &[Option<f64>]) -> Vec<Option<f64>> {
        match (self.center, self.cursor) {
            (Some(center), Some(cursor)) => resolved_circle(center, cursor, values)
                .map(|(_, radius)| vec![Some(radius)])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn dynamic_value_is_valid(&self, field_index: usize, value: f64) -> bool {
        match field_index {
            0 => value.is_finite() && value > f64::EPSILON,
            _ => true,
        }
    }

    fn on_dynamic_commit(
        &mut self,
        ctx: &ToolContext<'_>,
        world_mm: DVec2,
        values: &[Option<f64>],
    ) -> Vec<AppCommand> {
        let Some(center) = self.center else {
            return Vec::new();
        };
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };
        let Some((center, radius)) = resolved_circle(center, world_mm, values) else {
            return Vec::new();
        };
        self.center = None;
        self.cursor = None;
        vec![AppCommand::AddCircle {
            sketch,
            center,
            radius: LengthMm::new(radius),
        }]
    }
}

fn resolved_circle(center: DVec2, cursor: DVec2, values: &[Option<f64>]) -> Option<(DVec2, f64)> {
    let radius = values
        .first()
        .copied()
        .flatten()
        .unwrap_or_else(|| center.distance(cursor));

    if !radius.is_finite() || radius <= f64::EPSILON {
        return None;
    }

    Some((center, radius))
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_core::units::LengthMm;
    use roncad_geometry::Project;

    use super::CircleTool;
    use crate::tool::{Tool, ToolContext, ToolPreview};

    #[test]
    fn dynamic_commit_uses_typed_radius() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = CircleTool::default();

        tool.on_pointer_click(&ctx, dvec2(2.0, 3.0));
        // Cursor at distance 5 from center, typed radius overrides.
        let commands = tool.on_dynamic_commit(&ctx, dvec2(5.0, 7.0), &[Some(12.5)]);

        assert_eq!(
            commands,
            vec![AppCommand::AddCircle {
                sketch,
                center: dvec2(2.0, 3.0),
                radius: LengthMm::new(12.5),
            }]
        );
        assert!(matches!(tool.preview(), ToolPreview::None));
    }

    #[test]
    fn dynamic_commit_falls_back_to_cursor_distance() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = CircleTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        let commands = tool.on_dynamic_commit(&ctx, dvec2(3.0, 4.0), &[None]);

        assert_eq!(
            commands,
            vec![AppCommand::AddCircle {
                sketch,
                center: dvec2(0.0, 0.0),
                radius: LengthMm::new(5.0),
            }]
        );
    }

    #[test]
    fn right_click_clears_staged_circle() {
        let project = Project::new_untitled();
        let ctx = ToolContext {
            active_sketch: project.active_sketch,
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Default::default(),
        };
        let mut tool = CircleTool::default();

        tool.on_pointer_click(&ctx, dvec2(1.0, 1.0));
        tool.on_pointer_move(&ctx, dvec2(3.0, 1.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(3.0, 1.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }
}
