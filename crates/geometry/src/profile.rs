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
    profiles.extend(line_loop_profiles(sketch));
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
            SketchEntity::Rectangle { corner_a, corner_b } => {
                let min = corner_a.min(*corner_b);
                let max = corner_a.max(*corner_b);
                Some(SketchProfile::Polygon {
                    points: vec![
                        DVec2::new(min.x, min.y),
                        DVec2::new(max.x, min.y),
                        DVec2::new(max.x, max.y),
                        DVec2::new(min.x, max.y),
                    ],
                })
            }
            SketchEntity::Circle { center, radius } => Some(SketchProfile::Circle {
                center: *center,
                radius: *radius,
            }),
            SketchEntity::Point { .. } | SketchEntity::Line { .. } => None,
        })
        .collect()
}

fn line_loop_profiles(sketch: &Sketch) -> Vec<SketchProfile> {
    let mut vertices = Vec::<DVec2>::new();
    let mut vertex_ids = HashMap::<QuantizedPoint, usize>::new();
    let mut edges = Vec::<(usize, usize)>::new();

    for (_, entity) in sketch.iter() {
        let SketchEntity::Line { a, b } = entity else {
            continue;
        };
        let start = vertex_index(*a, &mut vertex_ids, &mut vertices);
        let end = vertex_index(*b, &mut vertex_ids, &mut vertices);
        if start == end {
            continue;
        }
        edges.push((start, end));
    }

    if edges.is_empty() {
        return Vec::new();
    }

    let mut adjacency = vec![Vec::<usize>::new(); vertices.len()];
    for (edge_id, (start, end)) in edges.iter().enumerate() {
        adjacency[*start].push(edge_id);
        adjacency[*end].push(edge_id);
    }

    let mut visited_edges = vec![false; edges.len()];
    let mut profiles = Vec::new();

    for edge_id in 0..edges.len() {
        if visited_edges[edge_id] {
            continue;
        }
        let (component_edges, component_vertices) =
            collect_component(edge_id, &edges, &adjacency, &mut visited_edges);

        if component_edges.len() < 3 || component_edges.len() != component_vertices.len() {
            continue;
        }
        if component_vertices
            .iter()
            .any(|vertex| component_degree(*vertex, &adjacency, &component_edges) != 2)
        {
            continue;
        }

        let points = order_cycle(&vertices, &edges, &adjacency, &component_vertices);
        if points.len() >= 3 && polygon_area(&points).abs() > PROFILE_EPSILON {
            profiles.push(SketchProfile::Polygon { points });
        }
    }

    profiles
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

fn collect_component(
    start_edge: usize,
    edges: &[(usize, usize)],
    adjacency: &[Vec<usize>],
    visited_edges: &mut [bool],
) -> (HashSet<usize>, HashSet<usize>) {
    let mut stack = vec![start_edge];
    let mut component_edges = HashSet::new();
    let mut component_vertices = HashSet::new();

    while let Some(edge_id) = stack.pop() {
        if visited_edges[edge_id] {
            continue;
        }
        visited_edges[edge_id] = true;
        component_edges.insert(edge_id);

        let (start, end) = edges[edge_id];
        for vertex in [start, end] {
            component_vertices.insert(vertex);
            for &next_edge in &adjacency[vertex] {
                if !visited_edges[next_edge] {
                    stack.push(next_edge);
                }
            }
        }
    }

    (component_edges, component_vertices)
}

fn component_degree(
    vertex: usize,
    adjacency: &[Vec<usize>],
    component_edges: &HashSet<usize>,
) -> usize {
    adjacency[vertex]
        .iter()
        .filter(|edge_id| component_edges.contains(edge_id))
        .count()
}

fn order_cycle(
    vertices: &[DVec2],
    edges: &[(usize, usize)],
    adjacency: &[Vec<usize>],
    component_vertices: &HashSet<usize>,
) -> Vec<DVec2> {
    let Some(&start) = component_vertices.iter().min() else {
        return Vec::new();
    };

    let mut ordered = Vec::with_capacity(component_vertices.len());
    let mut previous = None;
    let mut current = start;

    for _ in 0..=component_vertices.len() {
        ordered.push(vertices[current]);

        let next = adjacency[current]
            .iter()
            .filter_map(|edge_id| {
                let (a, b) = edges[*edge_id];
                let neighbor = if a == current { b } else { a };
                (Some(neighbor) != previous).then_some(neighbor)
            })
            .next();

        let Some(next_vertex) = next else {
            return Vec::new();
        };
        if next_vertex == start {
            break;
        }
        previous = Some(current);
        current = next_vertex;
    }

    ordered
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
