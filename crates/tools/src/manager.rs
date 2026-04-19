//! Routes the active ActiveToolKind to a concrete Tool implementation.
//! The shell owns a ToolManager and the viewport drives it with events.

use glam::DVec2;
use roncad_core::command::AppCommand;

use crate::circle_tool::CircleTool;
use crate::dimension_tool::DimensionTool;
use crate::fillet_tool::FilletTool;
use crate::line_tool::LineTool;
use crate::rectangle_tool::RectangleTool;
use crate::select_tool::SelectTool;
use crate::tool::{ActiveToolKind, Tool, ToolContext, ToolPreview};

pub struct ToolManager {
    active_kind: ActiveToolKind,
    tool: Box<dyn Tool>,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            active_kind: ActiveToolKind::Select,
            tool: Box::new(SelectTool::default()),
        }
    }

    pub fn active_kind(&self) -> ActiveToolKind {
        self.active_kind
    }

    pub fn set_active(&mut self, kind: ActiveToolKind) {
        if kind == self.active_kind {
            return;
        }
        self.tool.on_escape();
        self.active_kind = kind;
        self.tool = make_tool(kind);
    }

    pub fn on_pointer_move(&mut self, ctx: &ToolContext, world_mm: DVec2) {
        self.tool.on_pointer_move(ctx, world_mm);
    }

    pub fn on_pointer_click(&mut self, ctx: &ToolContext, world_mm: DVec2) -> Vec<AppCommand> {
        self.tool.on_pointer_click(ctx, world_mm)
    }

    pub fn on_pointer_secondary_click(
        &mut self,
        ctx: &ToolContext,
        world_mm: DVec2,
    ) -> Vec<AppCommand> {
        self.tool.on_pointer_secondary_click(ctx, world_mm)
    }

    pub fn on_escape(&mut self) {
        self.tool.on_escape();
    }

    pub fn preview(&self) -> ToolPreview {
        self.tool.preview()
    }

    pub fn step_hint(&self) -> String {
        self.tool
            .step_hint()
            .unwrap_or_else(|| self.active_kind.hint().to_string())
    }
}

fn make_tool(kind: ActiveToolKind) -> Box<dyn Tool> {
    match kind {
        ActiveToolKind::Select => Box::new(SelectTool::default()),
        ActiveToolKind::Pan => Box::new(PassiveTool(kind)),
        ActiveToolKind::Line => Box::new(LineTool::default()),
        ActiveToolKind::Rectangle => Box::new(RectangleTool::default()),
        ActiveToolKind::Circle => Box::new(CircleTool::default()),
        ActiveToolKind::Fillet => Box::new(FilletTool::default()),
        ActiveToolKind::Dimension => Box::new(DimensionTool::default()),
        ActiveToolKind::Extrude => Box::new(PassiveTool(kind)),
    }
}

struct PassiveTool(ActiveToolKind);

impl Tool for PassiveTool {
    fn kind(&self) -> ActiveToolKind {
        self.0
    }
}
