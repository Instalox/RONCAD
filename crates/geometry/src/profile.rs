//! Closed sketch profile detection and picking.
//! This is the geometry-side foundation for extrusion and region-aware tools.

use glam::DVec2;

use crate::{Sketch, SketchTopology};

const PROFILE_EPSILON: f64 = 1e-6;

#[derive(Debug, Clone, PartialEq)]
pub enum SketchProfile {
    Polygon { points: Vec<DVec2> },
    Circle { center: DVec2, radius: f64 },
}

impl SketchProfile {
    pub fn area(&self) -> f64 {
        match self {
            Self::Polygon { points } => polygon_area(points).abs(),
            Self::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
        }
    }

    pub fn centroid(&self) -> DVec2 {
        match self {
            Self::Polygon { points } => polygon_centroid(points),
            Self::Circle { center, .. } => *center,
        }
    }

    pub fn contains_point(&self, point: DVec2) -> bool {
        match self {
            Self::Polygon { points } => polygon_contains_point(points, point),
            Self::Circle { center, radius } => point.distance(*center) <= *radius + PROFILE_EPSILON,
        }
    }
}

pub fn closed_profiles(sketch: &Sketch) -> Vec<SketchProfile> {
    SketchTopology::from_sketch(sketch)
        .profiles()
        .iter()
        .map(|entry| entry.profile.clone())
        .collect()
}

pub fn pick_closed_profile(sketch: &Sketch, point: DVec2) -> Option<SketchProfile> {
    SketchTopology::from_sketch(sketch)
        .profiles()
        .iter()
        .map(|entry| entry.profile.clone())
        .filter(|profile| profile.contains_point(point))
        .min_by(|lhs, rhs| lhs.area().total_cmp(&rhs.area()))
}

fn polygon_area(points: &[DVec2]) -> f64 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area * 0.5
}

fn polygon_centroid(points: &[DVec2]) -> DVec2 {
    let area = polygon_area(points);
    if area.abs() <= PROFILE_EPSILON {
        let sum = points
            .iter()
            .copied()
            .fold(DVec2::ZERO, |acc, point| acc + point);
        return sum / points.len().max(1) as f64;
    }

    let mut cx = 0.0;
    let mut cy = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let cross = a.x * b.y - b.x * a.y;
        cx += (a.x + b.x) * cross;
        cy += (a.y + b.y) * cross;
    }
    DVec2::new(cx / (6.0 * area), cy / (6.0 * area))
}

fn polygon_contains_point(points: &[DVec2], point: DVec2) -> bool {
    for i in 0..points.len() {
        if distance_point_segment(point, points[i], points[(i + 1) % points.len()])
            <= PROFILE_EPSILON
        {
            return true;
        }
    }

    let mut inside = false;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let intersects = ((a.y > point.y) != (b.y > point.y))
            && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x);
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn distance_point_segment(point: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= PROFILE_EPSILON {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / len2).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::ids::WorkplaneId;

    use super::{closed_profiles, pick_closed_profile, SketchProfile};
    use crate::{Sketch, SketchEntity};

    #[test]
    fn closed_line_loop_becomes_polygon_profile() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 8.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 8.0),
            b: dvec2(0.0, 8.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 8.0),
            b: dvec2(0.0, 0.0),
        });

        let profiles = closed_profiles(&sketch);

        assert_eq!(profiles.len(), 1);
        assert!(matches!(
            &profiles[0],
            SketchProfile::Polygon { points } if points.len() == 4
        ));
        assert_eq!(profiles[0].area(), 80.0);
    }

    #[test]
    fn rectangle_split_by_center_line_becomes_two_profiles() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Rectangle {
            corner_a: dvec2(0.0, 0.0),
            corner_b: dvec2(20.0, 10.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 10.0),
        });

        let mut areas: Vec<_> = closed_profiles(&sketch)
            .into_iter()
            .map(|profile| profile.area())
            .collect();
        areas.sort_by(|lhs, rhs| lhs.total_cmp(rhs));

        assert_eq!(areas, vec![100.0, 100.0]);
    }

    #[test]
    fn open_linework_does_not_create_profile() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(10.0, 0.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 8.0),
        });

        assert!(closed_profiles(&sketch).is_empty());
    }

    #[test]
    fn picks_smallest_profile_under_cursor() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Rectangle {
            corner_a: dvec2(0.0, 0.0),
            corner_b: dvec2(20.0, 20.0),
        });
        sketch.add(SketchEntity::Circle {
            center: dvec2(10.0, 10.0),
            radius: 4.0,
        });

        let picked = pick_closed_profile(&sketch, dvec2(10.0, 10.0)).expect("profile");

        assert!(matches!(
            picked,
            SketchProfile::Circle { center, radius }
                if center == dvec2(10.0, 10.0) && radius == 4.0
        ));
    }
}
