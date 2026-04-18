//! Closed sketch profile detection and picking.
//! This is the geometry-side foundation for extrusion and region-aware tools.

use std::collections::{HashMap, HashSet};

use glam::DVec2;

use crate::{Sketch, SketchEntity};

const PROFILE_EPSILON: f64 = 1e-6;
const PROFILE_QUANTIZE_SCALE: f64 = 1_000_000.0;

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
    let mut profiles = primitive_profiles(sketch);
    profiles.extend(planar_polygon_profiles(sketch));
    profiles
}

pub fn pick_closed_profile(sketch: &Sketch, point: DVec2) -> Option<SketchProfile> {
    closed_profiles(sketch)
        .into_iter()
        .filter(|profile| profile.contains_point(point))
        .min_by(|lhs, rhs| lhs.area().total_cmp(&rhs.area()))
}

fn primitive_profiles(sketch: &Sketch) -> Vec<SketchProfile> {
    sketch
        .iter()
        .filter_map(|(_, entity)| match entity {
            SketchEntity::Circle { center, radius } => Some(SketchProfile::Circle {
                center: *center,
                radius: *radius,
            }),
            SketchEntity::Point { .. }
            | SketchEntity::Line { .. }
            | SketchEntity::Rectangle { .. } => None,
        })
        .collect()
}

fn planar_polygon_profiles(sketch: &Sketch) -> Vec<SketchProfile> {
    let raw_segments = sketch_segments(sketch);
    let segments = split_segments(&raw_segments);
    if segments.is_empty() {
        return Vec::new();
    }

    let (vertices, undirected_edges) = build_graph(&segments);
    if undirected_edges.is_empty() {
        return Vec::new();
    }

    let (half_edges, _) = build_half_edges(&vertices, &undirected_edges);
    let mut visited = vec![false; half_edges.len()];
    let mut profiles = Vec::new();

    for start in 0..half_edges.len() {
        if visited[start] {
            continue;
        }

        let mut current = start;
        let mut face = Vec::new();
        loop {
            if visited[current] && current != start {
                face.clear();
                break;
            }
            visited[current] = true;
            face.push(vertices[half_edges[current].from]);
            current = half_edges[current].next;
            if current == start {
                break;
            }
            if face.len() > half_edges.len() {
                face.clear();
                break;
            }
        }

        if face.len() < 3 {
            continue;
        }

        simplify_polygon(&mut face);
        if face.len() < 3 {
            continue;
        }

        let area = polygon_area(&face);
        if area > PROFILE_EPSILON {
            profiles.push(SketchProfile::Polygon { points: face });
        }
    }

    profiles
}

fn sketch_segments(sketch: &Sketch) -> Vec<(DVec2, DVec2)> {
    let mut segments = Vec::new();
    for (_, entity) in sketch.iter() {
        match entity {
            SketchEntity::Line { a, b } => segments.push((*a, *b)),
            SketchEntity::Rectangle { corner_a, corner_b } => {
                let min = corner_a.min(*corner_b);
                let max = corner_a.max(*corner_b);
                let corners = [
                    DVec2::new(min.x, min.y),
                    DVec2::new(max.x, min.y),
                    DVec2::new(max.x, max.y),
                    DVec2::new(min.x, max.y),
                ];
                for i in 0..4 {
                    segments.push((corners[i], corners[(i + 1) % 4]));
                }
            }
            SketchEntity::Point { .. } | SketchEntity::Circle { .. } => {}
        }
    }
    segments
}

fn split_segments(raw_segments: &[(DVec2, DVec2)]) -> Vec<(DVec2, DVec2)> {
    let mut split_points: Vec<Vec<DVec2>> = raw_segments
        .iter()
        .map(|(start, end)| vec![*start, *end])
        .collect();

    for i in 0..raw_segments.len() {
        for j in (i + 1)..raw_segments.len() {
            let (a, b) = raw_segments[i];
            let (c, d) = raw_segments[j];
            let Some(hit) = segment_intersection(a, b, c, d) else {
                continue;
            };
            split_points[i].push(hit.point);
            split_points[j].push(hit.point);
        }
    }

    let mut segments = Vec::new();
    for (index, (start, end)) in raw_segments.iter().enumerate() {
        let mut points = split_points[index].clone();
        points.sort_by(|lhs, rhs| {
            segment_parameter(*start, *end, *lhs)
                .total_cmp(&segment_parameter(*start, *end, *rhs))
        });
        points.dedup_by(|lhs, rhs| lhs.distance_squared(*rhs) <= PROFILE_EPSILON * PROFILE_EPSILON);

        for window in points.windows(2) {
            let a = window[0];
            let b = window[1];
            if a.distance_squared(b) > PROFILE_EPSILON * PROFILE_EPSILON {
                segments.push((a, b));
            }
        }
    }

    segments
}

fn build_graph(
    segments: &[(DVec2, DVec2)],
) -> (Vec<DVec2>, Vec<(usize, usize)>) {
    let mut vertices = Vec::<DVec2>::new();
    let mut vertex_ids = HashMap::<QuantizedPoint, usize>::new();
    let mut edges = Vec::<(usize, usize)>::new();
    let mut seen_edges = HashSet::<(usize, usize)>::new();

    for (start, end) in segments {
        let a = vertex_index(*start, &mut vertex_ids, &mut vertices);
        let b = vertex_index(*end, &mut vertex_ids, &mut vertices);
        if a == b {
            continue;
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if seen_edges.insert(key) {
            edges.push(key);
        }
    }

    (vertices, edges)
}

fn vertex_index(
    point: DVec2,
    vertex_ids: &mut HashMap<QuantizedPoint, usize>,
    vertices: &mut Vec<DVec2>,
) -> usize {
    let key = QuantizedPoint::from_point(point);
    *vertex_ids.entry(key).or_insert_with(|| {
        let id = vertices.len();
        vertices.push(point);
        id
    })
}

#[derive(Debug, Clone, Copy)]
struct HalfEdge {
    from: usize,
    to: usize,
    reverse: usize,
    next: usize,
    angle: f64,
}

fn build_half_edges(
    vertices: &[DVec2],
    undirected_edges: &[(usize, usize)],
) -> (Vec<HalfEdge>, Vec<Vec<usize>>) {
    let mut half_edges = Vec::<HalfEdge>::with_capacity(undirected_edges.len() * 2);
    let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];

    for &(a, b) in undirected_edges {
        let forward = half_edges.len();
        let reverse = forward + 1;
        half_edges.push(HalfEdge {
            from: a,
            to: b,
            reverse,
            next: usize::MAX,
            angle: edge_angle(vertices[a], vertices[b]),
        });
        half_edges.push(HalfEdge {
            from: b,
            to: a,
            reverse: forward,
            next: usize::MAX,
            angle: edge_angle(vertices[b], vertices[a]),
        });
        outgoing[a].push(forward);
        outgoing[b].push(reverse);
    }

    for edges in &mut outgoing {
        edges.sort_by(|lhs, rhs| half_edges[*lhs].angle.total_cmp(&half_edges[*rhs].angle));
    }

    for edge_id in 0..half_edges.len() {
        let to = half_edges[edge_id].to;
        let reverse = half_edges[edge_id].reverse;
        let edges = &outgoing[to];
        let pos = edges
            .iter()
            .position(|candidate| *candidate == reverse)
            .expect("reverse half-edge is present at destination");
        let next = edges[(pos + edges.len() - 1) % edges.len()];
        half_edges[edge_id].next = next;
    }

    (half_edges, outgoing)
}

fn edge_angle(a: DVec2, b: DVec2) -> f64 {
    let delta = b - a;
    delta.y.atan2(delta.x)
}

#[derive(Debug, Clone, Copy)]
struct SegmentIntersection {
    point: DVec2,
}

fn segment_intersection(
    a: DVec2,
    b: DVec2,
    c: DVec2,
    d: DVec2,
) -> Option<SegmentIntersection> {
    let r = b - a;
    let s = d - c;
    let denom = cross(r, s);
    if denom.abs() <= PROFILE_EPSILON {
        return None;
    }

    let q = c - a;
    let t_ab = cross(q, s) / denom;
    let t_cd = cross(q, r) / denom;
    if !(-PROFILE_EPSILON..=1.0 + PROFILE_EPSILON).contains(&t_ab)
        || !(-PROFILE_EPSILON..=1.0 + PROFILE_EPSILON).contains(&t_cd)
    {
        return None;
    }

    Some(SegmentIntersection {
        point: a + r * t_ab.clamp(0.0, 1.0),
    })
}

fn segment_parameter(a: DVec2, b: DVec2, point: DVec2) -> f64 {
    let delta = b - a;
    if delta.x.abs() >= delta.y.abs() && delta.x.abs() > PROFILE_EPSILON {
        (point.x - a.x) / delta.x
    } else if delta.y.abs() > PROFILE_EPSILON {
        (point.y - a.y) / delta.y
    } else {
        0.0
    }
}

fn simplify_polygon(points: &mut Vec<DVec2>) {
    let mut index = 0;
    while index < points.len() && points.len() >= 3 {
        let prev = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let collinear = cross(current - prev, next - current).abs() <= PROFILE_EPSILON;
        let between = (current.distance_squared(prev) > PROFILE_EPSILON * PROFILE_EPSILON)
            && (current.distance_squared(next) > PROFILE_EPSILON * PROFILE_EPSILON);
        if collinear && between {
            points.remove(index);
        } else {
            index += 1;
        }
    }
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
        let sum = points.iter().copied().fold(DVec2::ZERO, |acc, point| acc + point);
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
        if distance_point_segment(point, points[i], points[(i + 1) % points.len()]) <= PROFILE_EPSILON {
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

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct QuantizedPoint(i64, i64);

impl QuantizedPoint {
    fn from_point(point: DVec2) -> Self {
        Self(
            (point.x * PROFILE_QUANTIZE_SCALE).round() as i64,
            (point.y * PROFILE_QUANTIZE_SCALE).round() as i64,
        )
    }
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::ids::WorkplaneId;

    use super::{SketchProfile, closed_profiles, pick_closed_profile};
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
