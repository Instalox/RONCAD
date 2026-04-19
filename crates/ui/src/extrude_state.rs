use roncad_core::ids::SketchId;
use roncad_geometry::SketchProfile;
use roncad_tools::ActiveToolKind;

#[derive(Debug, Clone)]
pub struct ExtrudeDraft {
    pub sketch: SketchId,
    pub profile: SketchProfile,
}

#[derive(Debug, Default)]
pub struct ExtrudeHudState {
    active: Option<ExtrudeDraft>,
    distance_text: String,
    request_focus: bool,
    request_select_all: bool,
}

impl ExtrudeHudState {
    pub fn sync_active_tool(&mut self, active_tool: ActiveToolKind) {
        if active_tool != ActiveToolKind::Extrude {
            self.clear();
        }
    }

    pub fn arm(&mut self, sketch: SketchId, profile: SketchProfile) {
        self.active = Some(ExtrudeDraft { sketch, profile });
        if self.distance_text.trim().is_empty() {
            self.distance_text = "10.000".to_string();
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

    pub fn active(&self) -> Option<&ExtrudeDraft> {
        self.active.as_ref()
    }

    pub fn active_profile(&self) -> Option<&SketchProfile> {
        self.active.as_ref().map(|draft| &draft.profile)
    }

    pub fn distance_text(&self) -> &str {
        &self.distance_text
    }

    pub fn distance_text_mut(&mut self) -> &mut String {
        &mut self.distance_text
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

    pub fn parsed_distance(&self) -> Option<f64> {
        let value = self.distance_text.trim().parse::<f64>().ok()?;
        (value.is_finite() && value > 0.0).then_some(value)
    }
}
