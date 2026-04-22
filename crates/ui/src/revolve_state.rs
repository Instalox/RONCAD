use glam::DVec2;
use roncad_core::ids::SketchId;
use roncad_geometry::SketchProfile;
use roncad_tools::ActiveToolKind;

#[derive(Debug, Clone)]
pub struct RevolveDraft {
    pub sketch: SketchId,
    pub profile: SketchProfile,
    pub axis_origin: DVec2,
    pub axis_dir: DVec2,
}

#[derive(Debug, Default)]
pub struct RevolveHudState {
    active: Option<RevolveDraft>,
    angle_text: String,
    request_focus: bool,
    request_select_all: bool,
}

impl RevolveHudState {
    pub fn sync_active_tool(&mut self, active_tool: ActiveToolKind) {
        if active_tool != ActiveToolKind::Revolve {
            self.clear();
        }
    }

    pub fn arm(&mut self, sketch: SketchId, profile: SketchProfile, axis_origin: DVec2, axis_dir: DVec2) {
        self.active = Some(RevolveDraft {
            sketch,
            profile,
            axis_origin,
            axis_dir,
        });
        if self.angle_text.trim().is_empty() {
            self.angle_text = "360.0".to_string();
        }
        self.request_focus = true;
        self.request_select_all = true;
    }

    pub fn clear(&mut self) {
        self.active = None;
        self.request_focus = false;
        self.request_select_all = false;
    }

    pub fn is_open(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<&RevolveDraft> {
        self.active.as_ref()
    }

    pub fn angle_text(&self) -> &str {
        &self.angle_text
    }

    pub fn angle_text_mut(&mut self) -> &mut String {
        &mut self.angle_text
    }

    pub fn take_focus_request(&mut self) -> bool {
        let requested = self.request_focus;
        self.request_focus = false;
        requested
    }

    pub fn take_select_all_request(&mut self) -> bool {
        let requested = self.request_select_all;
        self.request_select_all = false;
        requested
    }

    pub fn parsed_angle_rad(&self) -> Option<f64> {
        let value_deg = self.angle_text.trim().parse::<f64>().ok()?;
        if value_deg.is_finite() && value_deg.abs() > 0.0 {
            Some(value_deg.to_radians())
        } else {
            None
        }
    }
}
