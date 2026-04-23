//! Central viewport. Milestone 2 paints the grid, the sketch entities of the
//! active sketch, and the live preview from the active tool. The app crate
//! owns interaction policy and injects it here as a controller callback.

mod body_overlay;
mod constraint_overlay;
mod dimension_overlay;
mod dynamic_overlay;
mod extrude_overlay;
mod grid_overlay;
mod hud_overlay;
mod mini_hud;
mod nav_gizmo;
mod profile_overlay;
mod sketch_overlay;
mod snap_overlay;
mod tool_overlay;
pub mod wgpu_renderer;

use egui::{Color32, Frame, Mesh, Pos2, Rect, Sense, Shape, Ui, UiBuilder};
use glam::DVec2;
use roncad_geometry::{HoverTarget, Project, Workplane};

use crate::shell::{ShellContext, ShellResponse};
use crate::theme::ThemeColors;

#[derive(Default)]
pub struct ViewportInteractionState {
    pub hovered_target: Option<HoverTarget>,
}

pub(super) const COLOR_SKETCH: Color32 = Color32::from_rgb(0xE0, 0xE4, 0xEA);

pub type ViewportInteractionHandler = for<'a> fn(
    &Ui,
    &egui::Response,
    Rect,
    &mut ShellContext<'a>,
    &mut ShellResponse,
) -> ViewportInteractionState;

pub fn render_in_rect(
    ui: &mut Ui,
    rect: Rect,
    shell: &mut ShellContext<'_>,
    response: &mut ShellResponse,
    handle_interaction: ViewportInteractionHandler,
) {
    let mut viewport_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("viewport")
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    viewport_ui.expand_to_include_rect(rect);
    viewport_ui.set_clip_rect(rect);

    Frame::new()
        .fill(ThemeColors::BG_DEEP)
        .show(&mut viewport_ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
            shell
                .camera
                .update_viewport(DVec2::new(rect.width() as f64, rect.height() as f64));

            paint_viewport_backdrop(ui.painter(), rect);

            let interaction = handle_interaction(ui, &resp, rect, shell, response);
            grid_overlay::paint(ui.painter(), rect, shell.camera, shell.project);
            body_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
            );
            sketch_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
                shell.last_solve_report,
                interaction.hovered_target.as_ref(),
                &response.highlighted_sketch_entities,
            );
            dimension_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.selection,
            );
            constraint_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project,
                shell.last_solve_report,
                response.highlighted_constraint,
            );
            profile_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project.active_workplane(),
                interaction
                    .hovered_target
                    .as_ref()
                    .and_then(HoverTarget::as_profile),
                shell.extrude_hud.active_profile(),
            );
            snap_overlay::paint(
                ui.painter(),
                rect,
                shell.camera,
                shell.project.active_workplane(),
                shell.snap_result.as_ref(),
            );
            tool_overlay::paint_preview(
                ui.painter(),
                rect,
                shell.camera,
                shell.project.active_workplane(),
                shell.tool_manager,
            );
            hud_overlay::paint(ui, rect, shell, interaction.hovered_target.as_ref());
            mini_hud::paint(ui, rect, shell, response);
            dynamic_overlay::paint(ui, rect, shell);
            extrude_overlay::paint(ui, rect, shell, response);
            nav_gizmo::paint(ui, rect, shell, response);
        });
}

/// Subtle radial backdrop: lightly lifted center, slightly deeper corners.
/// Painted as a 3x3 vertex-coloured mesh on top of the panel's flat fill so
/// the viewport reads as having depth, like a studio render.
fn paint_viewport_backdrop(painter: &egui::Painter, rect: Rect) {
    const CENTER: Color32 = Color32::from_rgb(0x1B, 0x1F, 0x27);
    const EDGE: Color32 = Color32::from_rgb(0x10, 0x12, 0x17);
    const CORNER: Color32 = Color32::from_rgb(0x08, 0x0A, 0x0D);

    let mid_x = rect.center().x;
    let mid_y = rect.center().y;
    let positions = [
        Pos2::new(rect.min.x, rect.min.y),
        Pos2::new(mid_x, rect.min.y),
        Pos2::new(rect.max.x, rect.min.y),
        Pos2::new(rect.min.x, mid_y),
        Pos2::new(mid_x, mid_y),
        Pos2::new(rect.max.x, mid_y),
        Pos2::new(rect.min.x, rect.max.y),
        Pos2::new(mid_x, rect.max.y),
        Pos2::new(rect.max.x, rect.max.y),
    ];
    let colors = [
        CORNER, EDGE, CORNER, EDGE, CENTER, EDGE, CORNER, EDGE, CORNER,
    ];

    let mut mesh = Mesh::default();
    mesh.reserve_vertices(9);
    mesh.reserve_triangles(8);
    for index in 0..9 {
        mesh.colored_vertex(positions[index], colors[index]);
    }
    // Two triangles per quad in the 3x3 grid.
    let quads = [(0u32, 1, 4, 3), (1, 2, 5, 4), (3, 4, 7, 6), (4, 5, 8, 7)];
    for (a, b, c, d) in quads {
        mesh.add_triangle(a, b, c);
        mesh.add_triangle(a, c, d);
    }
    painter.add(Shape::mesh(mesh));
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

pub(super) fn active_workplane<'a>(project: &'a Project) -> Option<&'a Workplane> {
    project.active_workplane()
}

pub(super) fn project_workplane_point(
    camera: &roncad_rendering::Camera2d,
    center: DVec2,
    workplane: &Workplane,
    point: DVec2,
) -> Option<Pos2> {
    camera
        .project_point(workplane.local_point(point), center)
        .map(to_pos)
}
