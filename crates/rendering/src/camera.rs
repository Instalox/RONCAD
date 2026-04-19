//! Perspective orbit camera used by the egui-painted viewport.
//! The legacy `Camera2d` name remains for now to limit churn across the app.

use glam::{DVec2, DVec3};

const MIN_PITCH_RADIANS: f64 = 5.0_f64.to_radians();
const MAX_PITCH_RADIANS: f64 = 85.0_f64.to_radians();
const MIN_ORBIT_RADIUS_MM: f64 = 8.0;
const MAX_ORBIT_RADIUS_MM: f64 = 100_000.0;
const NEAR_PLANE_MM: f64 = 0.1;
const ORBIT_RADIANS_PER_PIXEL: f64 = 0.01;

#[derive(Debug, Clone, Copy)]
pub struct Camera2d {
    pub pixels_per_mm: f64,
    target_mm: DVec3,
    yaw_radians: f64,
    pitch_radians: f64,
    orbit_radius_mm: f64,
    vertical_fov_radians: f64,
    viewport_size_px: DVec2,
}

impl Default for Camera2d {
    fn default() -> Self {
        let mut camera = Self {
            pixels_per_mm: 2.0,
            target_mm: DVec3::ZERO,
            yaw_radians: -35.0_f64.to_radians(),
            pitch_radians: 32.0_f64.to_radians(),
            orbit_radius_mm: 240.0,
            vertical_fov_radians: 40.0_f64.to_radians(),
            viewport_size_px: DVec2::new(1200.0, 800.0),
        };
        camera.refresh_pixels_per_mm();
        camera
    }
}

impl Camera2d {
    pub fn update_viewport(&mut self, screen_size_px: DVec2) {
        self.viewport_size_px = DVec2::new(screen_size_px.x.max(1.0), screen_size_px.y.max(1.0));
        self.refresh_pixels_per_mm();
    }

    pub fn world_to_screen(&self, world_mm: DVec2, screen_center: DVec2) -> DVec2 {
        self.project_point(DVec3::new(world_mm.x, world_mm.y, 0.0), screen_center)
            .unwrap_or(screen_center)
    }

    pub fn project_point(&self, world_mm: DVec3, screen_center: DVec2) -> Option<DVec2> {
        let (right, up, forward) = self.basis();
        let view = world_mm - self.eye_mm();
        let depth = view.dot(forward);
        if depth <= NEAR_PLANE_MM {
            return None;
        }

        let focal = self.focal_length_px();
        let x = view.dot(right) * focal / depth;
        let y = view.dot(up) * focal / depth;
        Some(DVec2::new(screen_center.x + x, screen_center.y - y))
    }

    pub fn screen_to_world(&self, screen: DVec2, screen_center: DVec2) -> DVec2 {
        self.screen_to_plane(screen, screen_center, 0.0)
            .unwrap_or_else(|| DVec2::new(self.target_mm.x, self.target_mm.y))
    }

    pub fn screen_to_plane(
        &self,
        screen: DVec2,
        screen_center: DVec2,
        plane_z_mm: f64,
    ) -> Option<DVec2> {
        let eye = self.eye_mm();
        let ray = self.screen_ray(screen, screen_center);
        if ray.z.abs() <= f64::EPSILON {
            return None;
        }

        let t = (plane_z_mm - eye.z) / ray.z;
        if t <= 0.0 {
            return None;
        }

        let point = eye + ray * t;
        Some(point.truncate())
    }

    pub fn zoom_about(&mut self, screen_point: DVec2, screen_center: DVec2, factor: f64) {
        let before = self.screen_to_plane(screen_point, screen_center, 0.0);
        self.orbit_radius_mm =
            (self.orbit_radius_mm / factor).clamp(MIN_ORBIT_RADIUS_MM, MAX_ORBIT_RADIUS_MM);
        if let Some(before) = before {
            if let Some(after) = self.screen_to_plane(screen_point, screen_center, 0.0) {
                let delta = before - after;
                self.target_mm.x += delta.x;
                self.target_mm.y += delta.y;
            }
        }
        self.refresh_pixels_per_mm();
    }

    pub fn pan_pixels(&mut self, delta_px: DVec2, screen_center: DVec2) {
        if delta_px.length_squared() <= f64::EPSILON {
            return;
        }

        let before = self.screen_to_plane(screen_center, screen_center, 0.0);
        let after = self.screen_to_plane(screen_center + delta_px, screen_center, 0.0);
        if let (Some(before), Some(after)) = (before, after) {
            let delta = before - after;
            self.target_mm.x += delta.x;
            self.target_mm.y += delta.y;
            self.refresh_pixels_per_mm();
        }
    }

    pub fn orbit_pixels(&mut self, delta_px: DVec2) {
        if delta_px.length_squared() <= f64::EPSILON {
            return;
        }

        self.yaw_radians -= delta_px.x * ORBIT_RADIANS_PER_PIXEL;
        self.pitch_radians = (self.pitch_radians + delta_px.y * ORBIT_RADIANS_PER_PIXEL)
            .clamp(MIN_PITCH_RADIANS, MAX_PITCH_RADIANS);
        self.refresh_pixels_per_mm();
    }

    pub fn fit_bounds_3d(
        &mut self,
        screen_size_px: DVec2,
        min_mm: DVec3,
        max_mm: DVec3,
        padding_px: f64,
    ) {
        self.update_viewport(screen_size_px);
        self.target_mm = (min_mm + max_mm) * 0.5;

        let usable_half_width = ((screen_size_px.x - padding_px * 2.0).max(32.0)) * 0.5;
        let usable_half_height = ((screen_size_px.y - padding_px * 2.0).max(32.0)) * 0.5;
        let focal = self.focal_length_px();
        let (right, up, forward) = self.basis();

        let corners = bounds_corners(min_mm, max_mm);
        let mut required_distance = MIN_ORBIT_RADIUS_MM;
        for corner in corners {
            let relative = corner - self.target_mm;
            let x = relative.dot(right).abs();
            let y = relative.dot(up).abs();
            let z = relative.dot(forward);
            required_distance = required_distance.max(focal * x / usable_half_width - z);
            required_distance = required_distance.max(focal * y / usable_half_height - z);
        }

        self.orbit_radius_mm = required_distance.clamp(MIN_ORBIT_RADIUS_MM, MAX_ORBIT_RADIUS_MM);
        self.refresh_pixels_per_mm();
    }

    pub fn eye_mm(&self) -> DVec3 {
        self.target_mm - self.forward_dir() * self.orbit_radius_mm
    }

    pub fn view_depth(&self, world_mm: DVec3) -> f64 {
        let (_, _, forward) = self.basis();
        (world_mm - self.eye_mm()).dot(forward)
    }

    pub fn plane_focus_mm(&self) -> DVec2 {
        DVec2::new(self.target_mm.x, self.target_mm.y)
    }

    pub fn plane_half_extents_mm(&self) -> DVec2 {
        let width = self.viewport_size_px.x / self.pixels_per_mm.max(f64::EPSILON) * 0.55;
        let height = self.viewport_size_px.y / self.pixels_per_mm.max(f64::EPSILON) * 0.55;
        DVec2::new(width.max(10.0), height.max(10.0))
    }

    fn screen_ray(&self, screen: DVec2, screen_center: DVec2) -> DVec3 {
        let focal = self.focal_length_px();
        let (right, up, forward) = self.basis();
        let x = (screen.x - screen_center.x) / focal;
        let y = -(screen.y - screen_center.y) / focal;
        (forward + right * x + up * y).normalize()
    }

    fn basis(&self) -> (DVec3, DVec3, DVec3) {
        let cos_pitch = self.pitch_radians.cos();
        let forward = DVec3::new(
            cos_pitch * self.yaw_radians.cos(),
            cos_pitch * self.yaw_radians.sin(),
            -self.pitch_radians.sin(),
        )
        .normalize();
        let world_up = DVec3::Z;
        let right = world_up.cross(forward).normalize();
        let up = forward.cross(right).normalize();
        (right, up, forward)
    }

    fn forward_dir(&self) -> DVec3 {
        self.basis().2
    }

    fn focal_length_px(&self) -> f64 {
        self.viewport_size_px.y.max(1.0) * 0.5 / (self.vertical_fov_radians * 0.5).tan()
    }

    fn refresh_pixels_per_mm(&mut self) {
        let screen_center = self.viewport_size_px * 0.5;
        let anchor = DVec3::new(self.target_mm.x, self.target_mm.y, 0.0);
        let axis = anchor + DVec3::X;

        self.pixels_per_mm = match (
            self.project_point(anchor, screen_center),
            self.project_point(axis, screen_center),
        ) {
            (Some(a), Some(b)) => (b - a).length().clamp(0.05, 10_000.0),
            _ => 1.0,
        };
    }
}

fn bounds_corners(min_mm: DVec3, max_mm: DVec3) -> [DVec3; 8] {
    [
        DVec3::new(min_mm.x, min_mm.y, min_mm.z),
        DVec3::new(max_mm.x, min_mm.y, min_mm.z),
        DVec3::new(min_mm.x, max_mm.y, min_mm.z),
        DVec3::new(max_mm.x, max_mm.y, min_mm.z),
        DVec3::new(min_mm.x, min_mm.y, max_mm.z),
        DVec3::new(max_mm.x, min_mm.y, max_mm.z),
        DVec3::new(min_mm.x, max_mm.y, max_mm.z),
        DVec3::new(max_mm.x, max_mm.y, max_mm.z),
    ]
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
    use glam::{dvec2, dvec3};

    use super::Camera2d;

    #[test]
    fn world_to_screen_and_back_round_trip_on_sketch_plane() {
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        let center = dvec2(400.0, 300.0);
        let point = dvec2(24.0, -18.0);

        let screen = camera.world_to_screen(point, center);
        let round_trip = camera.screen_to_world(screen, center);

        assert!((round_trip.x - point.x).abs() < 1e-6);
        assert!((round_trip.y - point.y).abs() < 1e-6);
    }

    #[test]
    fn fit_bounds_3d_centers_camera_on_bounds() {
        let mut camera = Camera2d::default();

        camera.fit_bounds_3d(
            dvec2(400.0, 300.0),
            dvec3(-10.0, -5.0, 0.0),
            dvec3(30.0, 15.0, 12.0),
            20.0,
        );

        assert!((camera.target_mm.x - 10.0).abs() < 1e-6);
        assert!((camera.target_mm.y - 5.0).abs() < 1e-6);
        assert!((camera.target_mm.z - 6.0).abs() < 1e-6);
        assert!(camera.orbit_radius_mm > 0.0);
        assert!(camera.pixels_per_mm > 0.0);
    }
}
