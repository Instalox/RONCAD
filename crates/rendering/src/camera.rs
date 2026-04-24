//! Perspective/orthographic orbit camera used by the egui-painted viewport.
//! The legacy `Camera2d` name remains for now to limit churn across the app.

use glam::{DMat4, DVec2, DVec3};
use roncad_geometry::Workplane;

const MIN_ORBIT_RADIUS_MM: f64 = 8.0;
const MAX_ORBIT_RADIUS_MM: f64 = 100_000.0;
const NEAR_PLANE_MM: f64 = 0.1;
const ORBIT_RADIANS_PER_PIXEL: f64 = 0.01;
const PITCH_LIMIT_RADIANS: f64 = 89.5_f64 * std::f64::consts::PI / 180.0;
const ANIMATION_DURATION_SECS: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy)]
pub struct Camera2d {
    pub pixels_per_mm: f64,
    target_mm: DVec3,
    yaw_radians: f64,
    pitch_radians: f64,
    orbit_radius_mm: f64,
    vertical_fov_radians: f64,
    viewport_size_px: DVec2,
    scale_axis_u_world: DVec3,
    scale_axis_v_world: DVec3,
    projection: Projection,
    animation: Option<OrientationAnimation>,
}

#[derive(Debug, Clone, Copy)]
struct OrientationAnimation {
    start_yaw: f64,
    start_pitch: f64,
    target_yaw: f64,
    target_pitch: f64,
    elapsed: f64,
    duration: f64,
}

impl Default for Camera2d {
    fn default() -> Self {
        let mut camera = Self {
            pixels_per_mm: 2.0,
            target_mm: DVec3::ZERO,
            yaw_radians: 120.0_f64.to_radians(),
            pitch_radians: 30.0_f64.to_radians(),
            orbit_radius_mm: 240.0,
            vertical_fov_radians: 40.0_f64.to_radians(),
            viewport_size_px: DVec2::new(1200.0, 800.0),
            scale_axis_u_world: DVec3::X,
            scale_axis_v_world: DVec3::Y,
            projection: Projection::Perspective,
            animation: None,
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

        match self.projection {
            Projection::Perspective => {
                if depth <= NEAR_PLANE_MM {
                    return None;
                }
                let focal = self.focal_length_px();
                let x = view.dot(right) * focal / depth;
                let y = view.dot(up) * focal / depth;
                Some(DVec2::new(screen_center.x + x, screen_center.y - y))
            }
            Projection::Orthographic => {
                let scale = self.ortho_pixels_per_mm();
                let x = view.dot(right) * scale;
                let y = view.dot(up) * scale;
                Some(DVec2::new(screen_center.x + x, screen_center.y - y))
            }
        }
    }

    pub fn screen_to_world(&self, screen: DVec2, screen_center: DVec2) -> DVec2 {
        self.screen_to_plane(screen, screen_center, 0.0)
            .unwrap_or_else(|| DVec2::new(self.target_mm.x, self.target_mm.y))
    }

    pub fn screen_to_workplane(
        &self,
        screen: DVec2,
        screen_center: DVec2,
        workplane: &Workplane,
    ) -> Option<DVec2> {
        let (origin, ray) = self.screen_ray_with_origin(screen, screen_center);
        let normal = workplane.normal();
        let denom = ray.dot(normal);
        if denom.abs() <= f64::EPSILON {
            return None;
        }

        let t = (workplane.origin - origin).dot(normal) / denom;
        if t <= 0.0 {
            return None;
        }

        let world = origin + ray * t;
        Some(workplane.world_to_local(world))
    }

    pub fn screen_to_plane(
        &self,
        screen: DVec2,
        screen_center: DVec2,
        plane_z_mm: f64,
    ) -> Option<DVec2> {
        let (origin, ray) = self.screen_ray_with_origin(screen, screen_center);
        if ray.z.abs() <= f64::EPSILON {
            return None;
        }

        let t = (plane_z_mm - origin.z) / ray.z;
        if t <= 0.0 {
            return None;
        }

        let point = origin + ray * t;
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

    pub fn zoom_about_workplane(
        &mut self,
        screen_point: DVec2,
        screen_center: DVec2,
        factor: f64,
        workplane: &Workplane,
    ) {
        let before = self.screen_to_workplane(screen_point, screen_center, workplane);
        self.orbit_radius_mm =
            (self.orbit_radius_mm / factor).clamp(MIN_ORBIT_RADIUS_MM, MAX_ORBIT_RADIUS_MM);
        if let Some(before) = before {
            if let Some(after) = self.screen_to_workplane(screen_point, screen_center, workplane) {
                self.target_mm += workplane_translation(workplane, before - after);
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

    pub fn pan_pixels_on_workplane(
        &mut self,
        delta_px: DVec2,
        screen_center: DVec2,
        workplane: &Workplane,
    ) {
        if delta_px.length_squared() <= f64::EPSILON {
            return;
        }

        let before = self.screen_to_workplane(screen_center, screen_center, workplane);
        let after = self.screen_to_workplane(screen_center + delta_px, screen_center, workplane);
        if let (Some(before), Some(after)) = (before, after) {
            self.target_mm += workplane_translation(workplane, before - after);
            self.refresh_pixels_per_mm();
        }
    }

    pub fn orbit_pixels(&mut self, delta_px: DVec2) {
        if delta_px.length_squared() <= f64::EPSILON {
            return;
        }

        self.yaw_radians += delta_px.x * ORBIT_RADIANS_PER_PIXEL;
        self.pitch_radians -= delta_px.y * ORBIT_RADIANS_PER_PIXEL;
        self.normalize_angles();
        self.refresh_pixels_per_mm();
    }

    pub fn orbit_radians(&mut self, yaw_delta: f64, pitch_delta: f64) {
        self.yaw_radians += yaw_delta;
        self.pitch_radians += pitch_delta;
        self.normalize_angles();
        self.refresh_pixels_per_mm();
    }

    pub fn dolly_step(&mut self, factor: f64) {
        if factor <= 0.0 || !factor.is_finite() {
            return;
        }
        self.orbit_radius_mm =
            (self.orbit_radius_mm / factor).clamp(MIN_ORBIT_RADIUS_MM, MAX_ORBIT_RADIUS_MM);
        self.refresh_pixels_per_mm();
    }

    pub fn set_orientation(&mut self, yaw_radians: f64, pitch_radians: f64) {
        self.animation = Some(OrientationAnimation {
            start_yaw: self.yaw_radians,
            start_pitch: self.pitch_radians,
            target_yaw: yaw_radians,
            target_pitch: pitch_radians,
            elapsed: 0.0,
            duration: ANIMATION_DURATION_SECS,
        });
    }

    /// Instantly set orientation without animation.
    pub fn set_orientation_immediate(&mut self, yaw_radians: f64, pitch_radians: f64) {
        self.animation = None;
        self.yaw_radians = yaw_radians;
        self.pitch_radians = pitch_radians;
        self.normalize_angles();
        self.refresh_pixels_per_mm();
    }

    /// Advance the orientation animation. Returns `true` while still animating.
    pub fn animate_step(&mut self, dt_seconds: f64) -> bool {
        let Some(ref mut anim) = self.animation else {
            return false;
        };
        anim.elapsed += dt_seconds;
        let t = (anim.elapsed / anim.duration).clamp(0.0, 1.0);
        // Smooth ease-out cubic: 1 - (1-t)^3
        let eased = 1.0 - (1.0 - t).powi(3);

        // Shortest-path yaw interpolation
        let mut dy = anim.target_yaw - anim.start_yaw;
        if dy > std::f64::consts::PI {
            dy -= std::f64::consts::TAU;
        } else if dy < -std::f64::consts::PI {
            dy += std::f64::consts::TAU;
        }
        self.yaw_radians = anim.start_yaw + dy * eased;
        self.pitch_radians = anim.start_pitch + (anim.target_pitch - anim.start_pitch) * eased;
        self.normalize_angles();
        self.refresh_pixels_per_mm();

        if t >= 1.0 {
            self.animation = None;
            false
        } else {
            true
        }
    }

    /// Whether the camera is currently animating.
    pub fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub fn yaw_radians(&self) -> f64 {
        self.yaw_radians
    }

    pub fn pitch_radians(&self) -> f64 {
        self.pitch_radians
    }

    pub fn projection(&self) -> Projection {
        self.projection
    }

    pub fn set_projection(&mut self, projection: Projection) {
        self.projection = projection;
        self.refresh_pixels_per_mm();
    }

    pub fn toggle_projection(&mut self) {
        self.projection = match self.projection {
            Projection::Perspective => Projection::Orthographic,
            Projection::Orthographic => Projection::Perspective,
        };
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
            match self.projection {
                Projection::Perspective => {
                    required_distance = required_distance.max(focal * x / usable_half_width - z);
                    required_distance = required_distance.max(focal * y / usable_half_height - z);
                }
                Projection::Orthographic => {
                    required_distance = required_distance.max(focal * x / usable_half_width);
                    required_distance = required_distance.max(focal * y / usable_half_height);
                }
            }
        }

        self.orbit_radius_mm = required_distance.clamp(MIN_ORBIT_RADIUS_MM, MAX_ORBIT_RADIUS_MM);
        self.refresh_pixels_per_mm();
    }

    pub fn align_to_workplane(&mut self, workplane: &Workplane) {
        let forward = -workplane.normal();
        self.yaw_radians = forward.y.atan2(forward.x);
        self.pitch_radians = (-forward.z).asin();
        self.target_mm = workplane.origin;
        self.scale_axis_u_world = workplane.u.normalize();
        self.scale_axis_v_world = workplane.v.normalize();
        self.normalize_angles();
        self.refresh_pixels_per_mm();
    }

    pub fn eye_mm(&self) -> DVec3 {
        self.target_mm - self.forward_dir() * self.orbit_radius_mm
    }

    pub fn view_depth(&self, world_mm: DVec3) -> f64 {
        let (_, _, forward) = self.basis();
        (world_mm - self.eye_mm()).dot(forward)
    }

    /// Right-handed view matrix; world → camera space.
    pub fn view_matrix(&self) -> DMat4 {
        let (_, up, _) = self.basis();
        DMat4::look_at_rh(self.eye_mm(), self.target_mm, up)
    }

    /// Projection matrix targeting wgpu/Vulkan/D3D NDC (z ∈ [0, 1]).
    pub fn projection_matrix(&self, viewport_size_px: DVec2) -> DMat4 {
        let aspect = (viewport_size_px.x / viewport_size_px.y.max(1.0)).max(0.001);
        let near = (self.orbit_radius_mm * 0.005).max(0.05);
        let far = (self.orbit_radius_mm * 16.0).max(near + 1.0);
        match self.projection {
            Projection::Perspective => {
                DMat4::perspective_rh(self.vertical_fov_radians, aspect, near, far)
            }
            Projection::Orthographic => {
                let half_h = self.orbit_radius_mm * (self.vertical_fov_radians * 0.5).tan();
                let half_w = half_h * aspect;
                DMat4::orthographic_rh(-half_w, half_w, -half_h, half_h, near, far)
            }
        }
    }

    /// Combined view-projection, packed as a column-major f32 matrix ready
    /// for upload to a wgpu uniform buffer.
    pub fn view_proj_f32(&self, viewport_size_px: DVec2) -> [[f32; 4]; 4] {
        (self.projection_matrix(viewport_size_px) * self.view_matrix())
            .as_mat4()
            .to_cols_array_2d()
    }

    pub fn plane_focus_mm(&self) -> DVec2 {
        DVec2::new(self.target_mm.x, self.target_mm.y)
    }

    pub fn plane_half_extents_mm(&self) -> DVec2 {
        let width = self.viewport_size_px.x / self.pixels_per_mm.max(f64::EPSILON) * 0.55;
        let height = self.viewport_size_px.y / self.pixels_per_mm.max(f64::EPSILON) * 0.55;
        DVec2::new(width.max(10.0), height.max(10.0))
    }

    pub fn viewport_size_px(&self) -> DVec2 {
        self.viewport_size_px
    }

    pub fn screen_ray_with_origin(&self, screen: DVec2, screen_center: DVec2) -> (DVec3, DVec3) {
        let (right, up, forward) = self.basis();
        match self.projection {
            Projection::Perspective => {
                let focal = self.focal_length_px();
                let x = (screen.x - screen_center.x) / focal;
                let y = -(screen.y - screen_center.y) / focal;
                (self.eye_mm(), (forward + right * x + up * y).normalize())
            }
            Projection::Orthographic => {
                let scale = self.ortho_pixels_per_mm().max(f64::EPSILON);
                let x_mm = (screen.x - screen_center.x) / scale;
                let y_mm = -(screen.y - screen_center.y) / scale;
                (self.eye_mm() + right * x_mm + up * y_mm, forward)
            }
        }
    }

    fn ortho_pixels_per_mm(&self) -> f64 {
        self.focal_length_px() / self.orbit_radius_mm.max(MIN_ORBIT_RADIUS_MM)
    }

    fn normalize_angles(&mut self) {
        let two_pi = std::f64::consts::TAU;
        self.yaw_radians = self.yaw_radians.rem_euclid(two_pi);
        self.pitch_radians = self
            .pitch_radians
            .clamp(-PITCH_LIMIT_RADIANS, PITCH_LIMIT_RADIANS);
    }

    fn basis(&self) -> (DVec3, DVec3, DVec3) {
        // Analytic Z-up turntable basis; well-defined at every pitch including
        // ±π/2, so the camera can pass through the poles without a fallback hack.
        let cos_pitch = self.pitch_radians.cos();
        let sin_pitch = self.pitch_radians.sin();
        let cos_yaw = self.yaw_radians.cos();
        let sin_yaw = self.yaw_radians.sin();
        let forward = DVec3::new(cos_yaw * cos_pitch, sin_yaw * cos_pitch, -sin_pitch);
        let right = DVec3::new(sin_yaw, -cos_yaw, 0.0);
        let up = DVec3::new(cos_yaw * sin_pitch, sin_yaw * sin_pitch, cos_pitch);
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
        let anchor = self.target_mm;
        let u_axis = anchor + self.scale_axis_u_world.normalize();
        let v_axis = anchor + self.scale_axis_v_world.normalize();

        let anchor_screen = self.project_point(anchor, screen_center);
        let u_px = anchor_screen
            .zip(self.project_point(u_axis, screen_center))
            .map(|(a, b)| (b - a).length());
        let v_px = anchor_screen
            .zip(self.project_point(v_axis, screen_center))
            .map(|(a, b)| (b - a).length());

        self.pixels_per_mm = u_px
            .into_iter()
            .chain(v_px)
            .fold(1.0, f64::max)
            .clamp(0.05, 10_000.0);
    }
}

fn workplane_translation(workplane: &Workplane, delta_mm: DVec2) -> DVec3 {
    workplane.u.normalize() * delta_mm.x + workplane.v.normalize() * delta_mm.y
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
    use std::f64::consts::FRAC_PI_2;

    use glam::{dvec2, dvec3};

    use super::{Camera2d, Projection};

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

    #[test]
    fn ortho_projection_preserves_apparent_size_at_target() {
        // Near-top view: pitch clamps at 89.5° so there's a tiny off-axis
        // component. Persp and ortho should still agree within a loose tolerance
        // for points at the target depth.
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        camera.set_orientation_immediate(FRAC_PI_2, FRAC_PI_2);
        let center = dvec2(400.0, 300.0);
        let world_point = dvec3(7.5, -3.25, 0.0);

        let persp = camera.project_point(world_point, center).unwrap();
        camera.set_projection(Projection::Orthographic);
        let ortho = camera.project_point(world_point, center).unwrap();

        assert!((persp.x - ortho.x).abs() < 1.0);
        assert!((persp.y - ortho.y).abs() < 1.0);
    }

    #[test]
    fn top_view_orientation_matches_blender_convention() {
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        camera.set_orientation_immediate(FRAC_PI_2, FRAC_PI_2);
        let center = dvec2(400.0, 300.0);

        let plus_x = camera.project_point(dvec3(10.0, 0.0, 0.0), center).unwrap();
        let plus_y = camera.project_point(dvec3(0.0, 10.0, 0.0), center).unwrap();

        // +X should appear to the right of center, +Y should appear above center
        // (note: egui screen y grows downward, so "above" means smaller y).
        assert!(plus_x.x > center.x);
        assert!((plus_x.y - center.y).abs() < 1e-3);
        assert!(plus_y.y < center.y);
        assert!((plus_y.x - center.x).abs() < 1e-3);
    }

    #[test]
    fn orbit_past_pole_is_clamped_and_stable() {
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        // Drag MMB straight up by many pixels — pitch should clamp at the
        // limit without producing NaN projections.
        for _ in 0..400 {
            camera.orbit_pixels(dvec2(0.0, -50.0));
        }
        let center = dvec2(400.0, 300.0);
        let projected = camera.project_point(dvec3(5.0, 0.0, 0.0), center);
        assert!(projected.is_some());
        let p = projected.unwrap();
        assert!(p.x.is_finite() && p.y.is_finite());
        // Pitch should be clamped near the limit, not wrapped
        assert!(camera.pitch_radians().abs() <= FRAC_PI_2);
    }

    #[test]
    fn pitch_clamping_prevents_over_rotation() {
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        camera.set_orientation_immediate(0.0, 100.0_f64.to_radians());
        assert!(camera.pitch_radians() <= 89.5_f64.to_radians() + 1e-6);
        camera.set_orientation_immediate(0.0, -100.0_f64.to_radians());
        assert!(camera.pitch_radians() >= -89.5_f64.to_radians() - 1e-6);
    }

    #[test]
    fn animation_converges_to_target() {
        let mut camera = Camera2d::default();
        camera.update_viewport(dvec2(800.0, 600.0));
        camera.set_orientation(FRAC_PI_2, FRAC_PI_2);
        assert!(camera.is_animating());

        // Step well past the animation duration
        for _ in 0..20 {
            camera.animate_step(0.02);
        }

        assert!(!camera.is_animating());
        // Pitch is clamped to PITCH_LIMIT, which is < FRAC_PI_2
        assert!((camera.yaw_radians() - FRAC_PI_2).abs() < 1e-6);
    }
}
