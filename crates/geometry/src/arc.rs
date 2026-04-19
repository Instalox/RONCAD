//! Shared arc helpers for sketch entities.

use std::f64::consts::TAU;

use glam::DVec2;

const ARC_EPSILON: f64 = 1e-6;

pub fn arc_end_angle(start_angle: f64, sweep_angle: f64) -> f64 {
    start_angle + sweep_angle
}

pub fn arc_point(center: DVec2, radius: f64, angle: f64) -> DVec2 {
    DVec2::new(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

pub fn arc_start_point(center: DVec2, radius: f64, start_angle: f64) -> DVec2 {
    arc_point(center, radius, start_angle)
}

pub fn arc_end_point(center: DVec2, radius: f64, start_angle: f64, sweep_angle: f64) -> DVec2 {
    arc_point(center, radius, arc_end_angle(start_angle, sweep_angle))
}

pub fn arc_mid_point(center: DVec2, radius: f64, start_angle: f64, sweep_angle: f64) -> DVec2 {
    arc_point(center, radius, start_angle + sweep_angle * 0.5)
}

pub fn arc_contains_angle(angle: f64, start_angle: f64, sweep_angle: f64) -> bool {
    if sweep_angle.abs() >= TAU - ARC_EPSILON {
        return true;
    }
    if sweep_angle.abs() <= ARC_EPSILON {
        return normalize_positive(angle - start_angle).abs() <= ARC_EPSILON;
    }

    if sweep_angle >= 0.0 {
        normalize_positive(angle - start_angle) <= sweep_angle + ARC_EPSILON
    } else {
        normalize_positive(start_angle - angle) <= -sweep_angle + ARC_EPSILON
    }
}

pub fn arc_sample_points(
    center: DVec2,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    max_step_radians: f64,
) -> Vec<DVec2> {
    let step = max_step_radians.abs().max(ARC_EPSILON);
    let steps = (sweep_angle.abs() / step).ceil().max(1.0) as usize;

    (0..=steps)
        .map(|index| {
            let t = index as f64 / steps as f64;
            arc_point(center, radius, start_angle + sweep_angle * t)
        })
        .collect()
}

pub fn distance_to_arc(
    point: DVec2,
    center: DVec2,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
) -> f64 {
    let radial = point - center;
    let angle = radial.y.atan2(radial.x);
    let start = arc_start_point(center, radius, start_angle);
    let end = arc_end_point(center, radius, start_angle, sweep_angle);

    if arc_contains_angle(angle, start_angle, sweep_angle) {
        (point.distance(center) - radius)
            .abs()
            .min(point.distance(start).min(point.distance(end)))
    } else {
        point.distance(start).min(point.distance(end))
    }
}

fn normalize_positive(angle: f64) -> f64 {
    angle.rem_euclid(TAU)
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use glam::dvec2;

    use super::{arc_contains_angle, arc_end_point, arc_mid_point, arc_start_point};

    #[test]
    fn clockwise_arc_contains_expected_angles() {
        assert!(arc_contains_angle(0.0, FRAC_PI_2, -FRAC_PI_2));
        assert!(arc_contains_angle(FRAC_PI_2, FRAC_PI_2, -FRAC_PI_2));
        assert!(!arc_contains_angle(PI, FRAC_PI_2, -FRAC_PI_2));
    }

    #[test]
    fn start_mid_and_end_points_follow_sweep() {
        let center = dvec2(2.0, 3.0);
        let radius = 5.0;
        let start = arc_start_point(center, radius, 0.0);
        let mid = arc_mid_point(center, radius, 0.0, PI);
        let end = arc_end_point(center, radius, 0.0, PI);

        assert_eq!(start, dvec2(7.0, 3.0));
        assert!((mid.x - 2.0).abs() < 1e-6);
        assert!((mid.y - 8.0).abs() < 1e-6);
        assert!((end.x + 3.0).abs() < 1e-6);
        assert!((end.y - 3.0).abs() < 1e-6);
    }
}
