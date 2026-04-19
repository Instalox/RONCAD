//! Central viewport. Milestone 2 paints the grid, the sketch entities of the
//! active sketch, and the live preview from the active tool. The app crate
//! owns interaction policy and injects it here as a controller callback.

mod dimension_overlay;
mod dynamic_overlay;
mod grid_overlay;
mod hud_overlay;
mod profile_overlay;
mod sketch_overlay;
mod snap_overlay;
mod tool_overlay;

use egui::{CentralPanel, Color32, Frame, Pos2, Rect, Sense, Ui};
use glam::DVec2;
use roncad_core::ids::{SketchEntityId, SketchId};
use roncad_geometry::SketchProfile;

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

#[derive(Default)]
pub struct ViewportInteractionState {
    pub hovered_entity: Option<(SketchId, SketchEntityId)>,
    pub hovered_profile: Option<SketchProfile>,
}

pub(super) const COLOR_SKETCH: Color32 = Color32::from_rgb(0xE0, 0xE4, 0xEA);

pub type ViewportInteractionHandler = for<'a> fn(
    &Ui,
    &egui::Response,
    Rect,
    &mut ShellContext<'a>,
    &mut ShellResponse,
) -> ViewportInteractionState;

pub fn render(
    ui: &mut Ui,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
    handle_interaction: ViewportInteractionHandler,
) {
    CentralPanel::default()
        .frame(Frame::new().fill(ThemeColors::BG_DEEP))
        .show_inside(ui, |ui| {
            let available = ui.available_size_before_wrap();
            let (rect, resp) = ui.allocate_exact_size(available, Sense::click_and_drag());

            let interaction = handle_interaction(ui, &resp, rect, shell, response);
            grid_overlay::paint(ui.painter(), rect, shell.camera);
            sketch_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
                interaction.hovered_entity,
            );
            dimension_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
            );
            profile_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                interaction.hovered_profile.as_ref(),
            );
            snap_overlay::paint(ui.painter(), rect, shell.camera, shell.snap_result.as_ref());
            tool_overlay::paint_preview(ui.painter(), rect, shell.camera, shell.tool_manager);
            hud_overlay::paint(ui, rect, shell, interaction.hovered_entity);
            dynamic_overlay::paint(ui, rect, shell);
        });
}

fn pick_step(pixels_per_mm: f64, min_px: f64) -> f64 {
    let target_mm = min_px / pixels_per_mm.max(f64::EPSILON);
    let pow = target_mm.log10().ceil();
    let base = 10f64.powf(pow);
    for candidate in [base * 0.1, base * 0.2, base * 0.5, base] {
        if candidate * pixels_per_mm >= min_px {
            return candidate;
        }
    }
    base
}

pub(super) fn screen_center(rect: Rect) -> DVec2 {
    DVec2::new(
        (rect.min.x as f64 + rect.max.x as f64) * 0.5,
        (rect.min.y as f64 + rect.max.y as f64) * 0.5,
    )
}

pub(super) fn to_pos(v: DVec2) -> Pos2 {
    Pos2::new(v.x as f32, v.y as f32)
}
