//! 2D camera state for the Milestone 1 egui-painted viewport.
//! Will be generalized to a 3D orbit/pan/zoom camera when wgpu lands.

use glam::DVec2;

#[derive(Debug, Clone, Copy)]
pub struct Camera2d {
    pub center_mm: DVec2,
    pub pixels_per_mm: f64,
}

impl Default for Camera2d {
    fn default() -> Self {
        Self {
            center_mm: DVec2::ZERO,
            pixels_per_mm: 10.0,
        }
    }
}

impl Camera2d {
    pub fn world_to_screen(&self, world_mm: DVec2, screen_center: DVec2) -> DVec2 {
        let offset = (world_mm - self.center_mm) * self.pixels_per_mm;
        DVec2::new(screen_center.x + offset.x, screen_center.y - offset.y)
    }

    pub fn screen_to_world(&self, screen: DVec2, screen_center: DVec2) -> DVec2 {
        let dx = screen.x - screen_center.x;
        let dy = screen_center.y - screen.y;
        self.center_mm + DVec2::new(dx, dy) / self.pixels_per_mm
    }

    pub fn zoom_about(&mut self, screen_point: DVec2, screen_center: DVec2, factor: f64) {
        let world_before = self.screen_to_world(screen_point, screen_center);
        self.pixels_per_mm = (self.pixels_per_mm * factor).clamp(0.1, 10_000.0);
        let world_after = self.screen_to_world(screen_point, screen_center);
        self.center_mm += world_before - world_after;
    }

    pub fn pan_pixels(&mut self, delta_px: DVec2) {
        self.center_mm += DVec2::new(-delta_px.x, delta_px.y) / self.pixels_per_mm;
    }
}

/// Pick an adaptive grid step (mm) such that on-screen spacing stays at or
/// above `min_pixel_spacing`. Uses a 1-2-5-10 decade ladder so steps always
/// fall on visually clean numbers.
pub fn adaptive_grid_step_mm(pixels_per_mm: f64, min_pixel_spacing: f64) -> f64 {
    let target_mm = min_pixel_spacing / pixels_per_mm.max(f64::EPSILON);
    let decade = 10f64.powf(target_mm.log10().ceil());
    for candidate in [decade * 0.1, decade * 0.2, decade * 0.5, decade] {
        if candidate * pixels_per_mm >= min_pixel_spacing {
            return candidate;
        }
    }
    decade
}
