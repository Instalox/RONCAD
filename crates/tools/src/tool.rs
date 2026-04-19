//! The shared Tool contract. Interactive features implement this trait
//! so the manager can feed pointer events, gather commands, and draw previews.

use glam::DVec2;
use roncad_core::command::AppCommand;
use roncad_core::ids::SketchId;
use roncad_geometry::Sketch;

pub const ENTITY_PICK_RADIUS_PX: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveToolKind {
    Select,
    Pan,
    Line,
    Rectangle,
    Circle,
    Arc,
    Fillet,
    Dimension,
    Extrude,
}

impl ActiveToolKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Pan => "Pan",
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Circle => "Circle",
            Self::Arc => "Arc",
            Self::Fillet => "Fillet",
            Self::Dimension => "Dimension",
            Self::Extrude => "Extrude",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Select => "Click to select. Ctrl/Shift-click toggles. Del deletes selection.",
            Self::Pan => "Drag to pan the viewport.",
            Self::Line => "Click points to chain lines. Right-click or Esc ends the chain.",
            Self::Rectangle => {
                "Click two opposite corners. Hold Shift to lock square. Right-click or Esc cancels."
            }
            Self::Circle => "Click center, then rim. Right-click or Esc cancels.",
            Self::Arc => "Click center, then start point, then end point to place an arc.",
            Self::Fillet => {
                "Click a corner shared by two lines, then move to set radius and click to apply."
            }
            Self::Dimension => "Pick two points to dimension. Right-click or Esc clears.",
            Self::Extrude => "Hover a closed profile to preview extrusion target.",
        }
    }

    pub fn shortcut(self) -> Option<&'static str> {
        match self {
            Self::Select => Some("V"),
            Self::Line => Some("L"),
            Self::Rectangle => Some("R"),
            Self::Circle => Some("C"),
            Self::Arc => Some("A"),
            Self::Fillet => Some("F"),
            Self::Dimension => Some("D"),
            Self::Extrude => Some("E"),
            Self::Pan => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Clone, Copy)]
pub struct ToolContext<'a> {
    pub active_sketch: Option<SketchId>,
    pub sketch: Option<&'a Sketch>,
    pub pixels_per_mm: f64,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
pub enum ToolPreview {
    None,
    Line {
        start: DVec2,
        end: DVec2,
    },
    Rectangle {
        corner_a: DVec2,
        corner_b: DVec2,
    },
    Circle {
        center: DVec2,
        radius: f64,
    },
    ArcRadius {
        center: DVec2,
        radius: f64,
        rim: DVec2,
    },
    Arc {
        center: DVec2,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    FilletHover {
        corner: DVec2,
        trim_a: (DVec2, DVec2),
        trim_b: (DVec2, DVec2),
        center: DVec2,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
        max_radius: f64,
    },
    Fillet {
        trim_a: (DVec2, DVec2),
        trim_b: (DVec2, DVec2),
        center: DVec2,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    Measurement {
        start: DVec2,
        end: DVec2,
    },
}

pub trait Tool: Send {
    fn kind(&self) -> ActiveToolKind;

    fn on_pointer_move(&mut self, _ctx: &ToolContext<'_>, _world_mm: DVec2) {}

    fn on_pointer_click(&mut self, _ctx: &ToolContext<'_>, _world_mm: DVec2) -> Vec<AppCommand> {
        Vec::new()
    }

    fn on_pointer_secondary_click(
        &mut self,
        _ctx: &ToolContext<'_>,
        _world_mm: DVec2,
    ) -> Vec<AppCommand> {
        Vec::new()
    }

    fn on_escape(&mut self) {}

    fn preview(&self) -> ToolPreview {
        ToolPreview::None
    }

    fn step_hint(&self) -> Option<String> {
        None
    }
}
