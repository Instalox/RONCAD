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

    pub fn fit_bounds(
        &mut self,
        screen_size_px: DVec2,
        min_mm: DVec2,
        max_mm: DVec2,
        padding_px: f64,
    ) {
        let width_mm = (max_mm.x - min_mm.x).abs().max(1.0);
        let height_mm = (max_mm.y - min_mm.y).abs().max(1.0);
        let usable_width_px = (screen_size_px.x - padding_px * 2.0).max(32.0);
        let usable_height_px = (screen_size_px.y - padding_px * 2.0).max(32.0);

        self.center_mm = (min_mm + max_mm) * 0.5;
        self.pixels_per_mm = (usable_width_px / width_mm)
            .min(usable_height_px / height_mm)
            .clamp(0.1, 10_000.0);
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

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::Camera2d;

    #[test]
    fn fit_bounds_centers_and_scales_camera() {
        let mut camera = Camera2d::default();

        camera.fit_bounds(
            dvec2(400.0, 300.0),
            dvec2(-10.0, -5.0),
            dvec2(30.0, 15.0),
            20.0,
        );

        assert_eq!(camera.center_mm, dvec2(10.0, 5.0));
        assert!((camera.pixels_per_mm - 9.0).abs() < 1e-6);
    }
}
