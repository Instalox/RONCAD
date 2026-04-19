//! Line tool: click the first point, click the second to commit a line
//! segment. Chains consecutive segments (second click becomes next start).
//! Escape cancels.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::tool::{
    ActiveToolKind, DynamicField, Modifiers, Tool, ToolContext, ToolPreview, DYN_FIELD_ANGLE_DEG,
    DYN_FIELD_LENGTH,
};

const LINE_DYN_FIELDS: &[DynamicField] = &[DYN_FIELD_LENGTH, DYN_FIELD_ANGLE_DEG];

#[derive(Default)]
pub struct LineTool {
    first_point: Option<DVec2>,
    cursor: Option<DVec2>,
}

impl Tool for LineTool {
    fn kind(&self) -> ActiveToolKind {
        ActiveToolKind::Line
    }

    fn on_pointer_move(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) {
        self.cursor = Some(match self.first_point {
            Some(start) => constrained_endpoint(start, world_mm, ctx.modifiers),
            None => world_mm,
        });
    }

    fn on_pointer_click(&mut self, ctx: &ToolContext<'_>, world_mm: DVec2) -> Vec<AppCommand> {
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };

        match self.first_point {
            None => {
                self.first_point = Some(world_mm);
                self.cursor = Some(world_mm);
                Vec::new()
            }
            Some(start) => {
                let end = constrained_endpoint(start, world_mm, ctx.modifiers);
                if (start - end).length_squared() < f64::EPSILON {
                    return Vec::new();
                }
                let commands = vec![AppCommand::AddLine {
                    sketch,
                    a: start,
                    b: end,
                }];
                self.first_point = Some(end);
                self.cursor = Some(end);
                commands
            }
        }
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.first_point = None;
        self.cursor = None;
        Vec::new()
    }

    fn on_escape(&mut self) {
        self.first_point = None;
        self.cursor = None;
    }

    fn preview(&self) -> ToolPreview {
        match (self.first_point, self.cursor) {
            (Some(start), Some(end)) => ToolPreview::Line { start, end },
            _ => ToolPreview::None,
        }
    }

    fn step_hint(&self) -> Option<String> {
        Some(match self.first_point {
            None => "Click first point. Shortcut: L. Right-click or Esc clears.".to_string(),
            Some(_) => "Type length, Tab, angle, Enter — or click next point. Shift locks axis."
                .to_string(),
        })
    }

    fn dynamic_fields(&self) -> &'static [DynamicField] {
        if self.first_point.is_some() {
            LINE_DYN_FIELDS
        } else {
            &[]
        }
    }

    fn dynamic_preview(&self, values: &[Option<f64>]) -> Option<ToolPreview> {
        let (start, end, ..) = resolved_line(self.first_point?, self.cursor?, values)?;
        Some(ToolPreview::Line { start, end })
    }

    fn dynamic_display_values(&self, values: &[Option<f64>]) -> Vec<Option<f64>> {
        match (self.first_point, self.cursor) {
            (Some(start), Some(cursor)) => resolved_line(start, cursor, values)
                .map(|(_, _, length, angle_deg)| vec![Some(length), Some(angle_deg)])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn dynamic_value_is_valid(&self, field_index: usize, value: f64) -> bool {
        match field_index {
            0 => value.is_finite() && value > f64::EPSILON,
            1 => value.is_finite(),
            _ => true,
        }
    }

    fn on_dynamic_commit(
        &mut self,
        ctx: &ToolContext<'_>,
        world_mm: DVec2,
        values: &[Option<f64>],
    ) -> Vec<AppCommand> {
        let Some(start) = self.first_point else {
            return Vec::new();
        };
        let Some(sketch) = ctx.active_sketch else {
            return Vec::new();
        };
        let cursor = constrained_endpoint(start, world_mm, ctx.modifiers);
        let Some((_, end, ..)) = resolved_line(start, cursor, values) else {
            return Vec::new();
        };
        self.first_point = Some(end);
        self.cursor = Some(end);
        vec![AppCommand::AddLine {
            sketch,
            a: start,
            b: end,
        }]
    }
}

fn resolved_line(
    start: DVec2,
    cursor: DVec2,
    values: &[Option<f64>],
) -> Option<(DVec2, DVec2, f64, f64)> {
    let delta = cursor - start;
    let cursor_length = delta.length();
    let cursor_angle = if cursor_length > f64::EPSILON {
        delta.y.atan2(delta.x)
    } else {
        0.0
    };

    let length = values.first().copied().flatten().unwrap_or(cursor_length);
    let angle = values
        .get(1)
        .copied()
        .flatten()
        .map(f64::to_radians)
        .unwrap_or(cursor_angle);

    if !length.is_finite() || length <= f64::EPSILON || !angle.is_finite() {
        return None;
    }

    let end = start + DVec2::new(length * angle.cos(), length * angle.sin());
    Some((start, end, length, normalize_degrees(angle)))
}

fn normalize_degrees(angle_radians: f64) -> f64 {
    angle_radians.to_degrees().rem_euclid(360.0)
}

fn constrained_endpoint(start: DVec2, end: DVec2, modifiers: Modifiers) -> DVec2 {
    if !modifiers.shift {
        return end;
    }

    let delta = end - start;
    if delta.x.abs() >= delta.y.abs() {
        DVec2::new(end.x, start.y)
    } else {
        DVec2::new(start.x, end.y)
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::command::AppCommand;
    use roncad_geometry::Project;

    use super::LineTool;
    use crate::tool::{Modifiers, Tool, ToolContext, ToolPreview};

    #[test]
    fn shift_click_locks_line_to_dominant_axis() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        };
        let mut tool = LineTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        let commands = tool.on_pointer_click(&ctx, dvec2(10.0, 4.0));

        assert_eq!(
            commands,
            vec![AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(10.0, 0.0),
            }]
        );
    }

    #[test]
    fn dynamic_commit_uses_typed_length_and_angle() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = LineTool::default();

        tool.on_pointer_click(&ctx, dvec2(1.0, 2.0));
        let commands = tool.on_dynamic_commit(&ctx, dvec2(7.0, 5.0), &[Some(10.0), Some(90.0)]);

        let AppCommand::AddLine { sketch: s, a, b } = commands[0] else {
            panic!("expected AddLine command, got {:?}", commands);
        };
        assert_eq!(s, sketch);
        assert_eq!(a, dvec2(1.0, 2.0));
        assert!((b.x - 1.0).abs() < 1e-9, "b.x = {}", b.x);
        assert!((b.y - 12.0).abs() < 1e-9, "b.y = {}", b.y);
        // Chain continues from the committed endpoint.
        assert!(matches!(tool.preview(), ToolPreview::Line { .. }));
    }

    #[test]
    fn dynamic_commit_falls_back_to_cursor_angle_when_only_length_typed() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = LineTool::default();

        tool.on_pointer_click(&ctx, dvec2(0.0, 0.0));
        // Cursor is along +x, typed length overrides distance.
        let commands = tool.on_dynamic_commit(&ctx, dvec2(3.0, 0.0), &[Some(5.0), None]);

        assert_eq!(
            commands,
            vec![AppCommand::AddLine {
                sketch,
                a: dvec2(0.0, 0.0),
                b: dvec2(5.0, 0.0),
            }]
        );
    }

    #[test]
    fn right_click_ends_active_chain() {
        let project = Project::new_untitled();
        let sketch = project.active_sketch.expect("default sketch");
        let ctx = ToolContext {
            active_sketch: Some(sketch),
            sketch: project.active_sketch(),
            pixels_per_mm: 10.0,
            modifiers: Modifiers::default(),
        };
        let mut tool = LineTool::default();

        tool.on_pointer_click(&ctx, dvec2(1.0, 1.0));
        tool.on_pointer_move(&ctx, dvec2(5.0, 2.0));
        tool.on_pointer_secondary_click(&ctx, dvec2(5.0, 2.0));

        assert!(matches!(tool.preview(), ToolPreview::None));
    }
}
