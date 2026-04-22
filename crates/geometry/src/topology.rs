//! Derived sketch topology.
//!
//! Builds a normalized graph from raw sketch entities by splitting linework at
//! intersections, extracting closed faces, and attaching stable-ish profile
//! keys based on source entity spans.

use std::collections::HashMap;

use glam::DVec2;
use roncad_core::ids::SketchEntityId;
use slotmap::Key;

use crate::{arc_sample_points, profile::SketchProfile, Sketch, SketchEntity};

const TOPOLOGY_EPSILON: f64 = 1e-6;
const TOPOLOGY_POINT_SCALE: f64 = 1_000_000.0;
const TOPOLOGY_PARAM_SCALE: f64 = 100_000.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileSpanKey {
    pub entity: SketchEntityId,
    pub part: u32,
    pub start_param: i32,
    pub end_param: i32,
}

impl ProfileSpanKey {
    fn new(entity: SketchEntityId, part: u32, start_param: f64, end_param: f64) -> Self {
        let start = quantize_param(start_param.min(end_param));
        let end = quantize_param(start_param.max(end_param));
        Self {
            entity,
            part,
            start_param: start,
            end_param: end,
        }
    }

    fn sort_key(&self) -> (u64, u32, i32, i32) {
        (
            self.entity.data().as_ffi(),
            self.part,
            self.start_param,
            self.end_param,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileKey {
    pub spans: Vec<ProfileSpanKey>,
}

impl ProfileKey {
    fn new(mut spans: Vec<ProfileSpanKey>) -> Self {
        spans.sort_by_key(ProfileSpanKey::sort_key);
        spans.dedup();
        Self { spans }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologyProfile {
    pub key: ProfileKey,
    pub profile: SketchProfile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopologyEdge {
    pub start: usize,
    pub end: usize,
    pub span: ProfileSpanKey,
}

#[derive(Debug, Clone, Default)]
pub struct SketchTopology {
    vertices: Vec<DVec2>,
    edges: Vec<TopologyEdge>,
    profiles: Vec<TopologyProfile>,
}

impl SketchTopology {
    pub fn from_sketch(sketch: &Sketch) -> Self {
        let mut profiles = primitive_profiles(sketch);

        let raw_segments = sketch_segments(sketch);
        let segments = split_segments(&raw_segments);
        if segments.is_empty() {
            return Self {
                vertices: Vec::new(),
                edges: Vec::new(),
                profiles,
            };
        }

        let (vertices, edges) = build_graph(&segments);
        if !edges.is_empty() {
            profiles.extend(extract_polygon_profiles(&vertices, &edges));
        }

        Self {
            vertices,
            edges,
            profiles,
        }
    }

    pub fn vertices(&self) -> &[DVec2] {
        &self.vertices
    }

    pub fn edges(&self) -> &[TopologyEdge] {
        &self.edges
    }

    pub fn profiles(&self) -> &[TopologyProfile] {
        &self.profiles
    }

    pub fn find_profile(&self, profile: &SketchProfile) -> Option<&TopologyProfile> {
        self.profiles
            .iter()
            .find(|candidate| profiles_match(&candidate.profile, profile))
    }

    pub fn profile_by_key(&self, key: &ProfileKey) -> Option<&TopologyProfile> {
        self.profiles.iter().find(|candidate| &candidate.key == key)
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceSegment {
    start: DVec2,
    end: DVec2,
    entity: SketchEntityId,
    part: u32,
}

#[derive(Debug, Clone)]
struct SplitSegment {
    start: DVec2,
    end: DVec2,
    span: ProfileSpanKey,
}

#[derive(Debug, Clone, Copy)]
struct HalfEdge {
    from: usize,
    to: usize,
    reverse: usize,
    next: usize,
    angle: f64,
    span: usize,
}

#[derive(Debug, Clone, Copy)]
struct SegmentIntersection {
    point: DVec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct QuantizedPoint(i64, i64);

impl QuantizedPoint {
    fn from_point(point: DVec2) -> Self {
        Self(
            (point.x * TOPOLOGY_POINT_SCALE).round() as i64,
            (point.y * TOPOLOGY_POINT_SCALE).round() as i64,
        )
    }
}

fn primitive_profiles(sketch: &Sketch) -> Vec<TopologyProfile> {
    sketch
        .iter()
        .filter_map(|(id, entity)| match entity {
            SketchEntity::Circle { center, radius } => Some(TopologyProfile {
                key: ProfileKey::new(vec![ProfileSpanKey::new(id, 0, 0.0, 1.0)]),
                profile: SketchProfile::Circle {
                    center: *center,
                    radius: *radius,
                },
            }),
            SketchEntity::Point { .. }
            | SketchEntity::Line { .. }
            | SketchEntity::Rectangle { .. }
            | SketchEntity::Arc { .. } => None,
        })
        .collect()
}

fn sketch_segments(sketch: &Sketch) -> Vec<SourceSegment> {
    let mut segments = Vec::new();
    for (id, entity) in sketch.iter() {
        match entity {
            SketchEntity::Line { a, b } => segments.push(SourceSegment {
                start: *a,
                end: *b,
                entity: id,
                part: 0,
            }),
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
                    segments.push(SourceSegment {
                        start: corners[i],
                        end: corners[(i + 1) % 4],
                        entity: id,
                        part: i as u32,
                    });
                }
            }
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => {
                let points = arc_sample_points(
                    *center,
                    *radius,
                    *start_angle,
                    *sweep_angle,
                    std::f64::consts::PI / 48.0,
                );
                for (part, window) in points.windows(2).enumerate() {
                    segments.push(SourceSegment {
                        start: window[0],
                        end: window[1],
                        entity: id,
                        part: part as u32,
                    });
                }
            }
            SketchEntity::Point { .. } | SketchEntity::Circle { .. } => {}
        }
    }
    segments
}

fn split_segments(raw_segments: &[SourceSegment]) -> Vec<SplitSegment> {
    let mut split_points: Vec<Vec<DVec2>> = raw_segments
        .iter()
        .map(|segment| vec![segment.start, segment.end])
        .collect();

    for i in 0..raw_segments.len() {
        for j in (i + 1)..raw_segments.len() {
            let a = raw_segments[i];
            let b = raw_segments[j];
            let Some(hit) = segment_intersection(a.start, a.end, b.start, b.end) else {
                continue;
            };
            split_points[i].push(hit.point);
            split_points[j].push(hit.point);
        }
    }

    let mut segments = Vec::new();
    for (index, source) in raw_segments.iter().enumerate() {
        let mut points = split_points[index].clone();
        points.sort_by(|lhs, rhs| {
            segment_parameter(source.start, source.end, *lhs).total_cmp(&segment_parameter(
                source.start,
                source.end,
                *rhs,
            ))
        });
        points
            .dedup_by(|lhs, rhs| lhs.distance_squared(*rhs) <= TOPOLOGY_EPSILON * TOPOLOGY_EPSILON);

        for window in points.windows(2) {
            let start = window[0];
            let end = window[1];
            if start.distance_squared(end) <= TOPOLOGY_EPSILON * TOPOLOGY_EPSILON {
                continue;
            }

            segments.push(SplitSegment {
                start,
                end,
                span: ProfileSpanKey::new(
                    source.entity,
                    source.part,
                    segment_parameter(source.start, source.end, start),
                    segment_parameter(source.start, source.end, end),
                ),
            });
        }
    }

    segments
}

fn build_graph(segments: &[SplitSegment]) -> (Vec<DVec2>, Vec<TopologyEdge>) {
    let mut vertices = Vec::<DVec2>::new();
    let mut vertex_ids = HashMap::<QuantizedPoint, usize>::new();
    let mut edges = HashMap::<(usize, usize), ProfileSpanKey>::new();

    for segment in segments {
        let a = vertex_index(segment.start, &mut vertex_ids, &mut vertices);
        let b = vertex_index(segment.end, &mut vertex_ids, &mut vertices);
        if a == b {
            continue;
        }
        let key = if a < b { (a, b) } else { (b, a) };
        match edges.get(&key) {
            Some(existing) if existing.sort_key() <= segment.span.sort_key() => {}
            _ => {
                edges.insert(key, segment.span.clone());
            }
        }
    }

    let mut edges: Vec<_> = edges
        .into_iter()
        .map(|((start, end), span)| TopologyEdge { start, end, span })
        .collect();
    edges.sort_by(|lhs, rhs| {
        lhs.start
            .cmp(&rhs.start)
            .then(lhs.end.cmp(&rhs.end))
            .then(lhs.span.sort_key().cmp(&rhs.span.sort_key()))
    });

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

fn extract_polygon_profiles(vertices: &[DVec2], edges: &[TopologyEdge]) -> Vec<TopologyProfile> {
    let (half_edges, _) = build_half_edges(vertices, edges);
    let mut visited = vec![false; half_edges.len()];
    let mut profiles = Vec::new();

    for start in 0..half_edges.len() {
        if visited[start] {
            continue;
        }

        let mut current = start;
        let mut face = Vec::new();
        let mut spans = Vec::new();
        loop {
            if visited[current] && current != start {
                face.clear();
                spans.clear();
                break;
            }
            visited[current] = true;
            face.push(vertices[half_edges[current].from]);
            spans.push(edges[half_edges[current].span].span.clone());
            current = half_edges[current].next;
            if current == start {
                break;
            }
            if face.len() > half_edges.len() {
                face.clear();
                spans.clear();
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
        if area > TOPOLOGY_EPSILON {
            profiles.push(TopologyProfile {
                key: ProfileKey::new(spans),
                profile: SketchProfile::Polygon { points: face },
            });
        }
    }

    profiles
}

fn build_half_edges(
    vertices: &[DVec2],
    edges: &[TopologyEdge],
) -> (Vec<HalfEdge>, Vec<Vec<usize>>) {
    let mut half_edges = Vec::<HalfEdge>::with_capacity(edges.len() * 2);
    let mut outgoing = vec![Vec::<usize>::new(); vertices.len()];

    for (edge_index, edge) in edges.iter().enumerate() {
        let forward = half_edges.len();
        let reverse = forward + 1;
        half_edges.push(HalfEdge {
            from: edge.start,
            to: edge.end,
            reverse,
            next: usize::MAX,
            angle: edge_angle(vertices[edge.start], vertices[edge.end]),
            span: edge_index,
        });
        half_edges.push(HalfEdge {
            from: edge.end,
            to: edge.start,
            reverse: forward,
            next: usize::MAX,
            angle: edge_angle(vertices[edge.end], vertices[edge.start]),
            span: edge_index,
        });
        outgoing[edge.start].push(forward);
        outgoing[edge.end].push(reverse);
    }

    for edges_at_vertex in &mut outgoing {
        edges_at_vertex
            .sort_by(|lhs, rhs| half_edges[*lhs].angle.total_cmp(&half_edges[*rhs].angle));
    }

    for edge_id in 0..half_edges.len() {
        let to = half_edges[edge_id].to;
        let reverse = half_edges[edge_id].reverse;
        let edges_at_vertex = &outgoing[to];
        let position = edges_at_vertex
            .iter()
            .position(|candidate| *candidate == reverse)
            .expect("reverse half-edge exists at destination");
        let next = edges_at_vertex[(position + edges_at_vertex.len() - 1) % edges_at_vertex.len()];
        half_edges[edge_id].next = next;
    }

    (half_edges, outgoing)
}

fn edge_angle(a: DVec2, b: DVec2) -> f64 {
    let delta = b - a;
    delta.y.atan2(delta.x)
}

fn segment_intersection(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Option<SegmentIntersection> {
    let r = b - a;
    let s = d - c;
    let denom = cross(r, s);
    if denom.abs() <= TOPOLOGY_EPSILON {
        return None;
    }

    let q = c - a;
    let t_ab = cross(q, s) / denom;
    let t_cd = cross(q, r) / denom;
    if !(-TOPOLOGY_EPSILON..=1.0 + TOPOLOGY_EPSILON).contains(&t_ab)
        || !(-TOPOLOGY_EPSILON..=1.0 + TOPOLOGY_EPSILON).contains(&t_cd)
    {
        return None;
    }

    Some(SegmentIntersection {
        point: a + r * t_ab.clamp(0.0, 1.0),
    })
}

fn segment_parameter(a: DVec2, b: DVec2, point: DVec2) -> f64 {
    let delta = b - a;
    if delta.x.abs() >= delta.y.abs() && delta.x.abs() > TOPOLOGY_EPSILON {
        (point.x - a.x) / delta.x
    } else if delta.y.abs() > TOPOLOGY_EPSILON {
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
        let collinear = cross(current - prev, next - current).abs() <= TOPOLOGY_EPSILON;
        let separated = current.distance_squared(prev) > TOPOLOGY_EPSILON * TOPOLOGY_EPSILON
            && current.distance_squared(next) > TOPOLOGY_EPSILON * TOPOLOGY_EPSILON;
        if collinear && separated {
            points.remove(index);
        } else {
            index += 1;
        }
    }
}

fn quantize_param(value: f64) -> i32 {
    (value * TOPOLOGY_PARAM_SCALE).round() as i32
}

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
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

fn profiles_match(lhs: &SketchProfile, rhs: &SketchProfile) -> bool {
    match (lhs, rhs) {
        (
            SketchProfile::Circle {
                center: lhs_center,
                radius: lhs_radius,
            },
            SketchProfile::Circle {
                center: rhs_center,
                radius: rhs_radius,
            },
        ) => {
            lhs_center.distance_squared(*rhs_center) <= TOPOLOGY_EPSILON * TOPOLOGY_EPSILON
                && (lhs_radius - rhs_radius).abs() <= TOPOLOGY_EPSILON
        }
        (
            SketchProfile::Polygon { points: lhs_points },
            SketchProfile::Polygon { points: rhs_points },
        ) => polygon_matches(lhs_points, rhs_points),
        _ => false,
    }
}

fn polygon_matches(lhs: &[DVec2], rhs: &[DVec2]) -> bool {
    if lhs.len() != rhs.len() {
        return false;
    }
    if lhs.is_empty() {
        return true;
    }

    for start in 0..rhs.len() {
        if point_matches(lhs[0], rhs[start]) {
            let mut forward = true;
            for index in 0..lhs.len() {
                if !point_matches(lhs[index], rhs[(start + index) % rhs.len()]) {
                    forward = false;
                    break;
                }
            }
            if forward {
                return true;
            }

            let mut reverse = true;
            for index in 0..lhs.len() {
                let rhs_index = (start + rhs.len() - index) % rhs.len();
                if !point_matches(lhs[index], rhs[rhs_index]) {
                    reverse = false;
                    break;
                }
            }
            if reverse {
                return true;
            }
        }
    }

    false
}

fn point_matches(lhs: DVec2, rhs: DVec2) -> bool {
    lhs.distance_squared(rhs) <= TOPOLOGY_EPSILON * TOPOLOGY_EPSILON
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_core::ids::WorkplaneId;

    use super::SketchTopology;
    use crate::{profile::SketchProfile, Sketch, SketchEntity};

    #[test]
    fn topology_splits_crossing_segments_into_two_faces() {
        let mut sketch = Sketch::new("Sketch", WorkplaneId::default());
        sketch.add(SketchEntity::Rectangle {
            corner_a: dvec2(0.0, 0.0),
            corner_b: dvec2(20.0, 10.0),
        });
        sketch.add(SketchEntity::Line {
            a: dvec2(10.0, 0.0),
            b: dvec2(10.0, 10.0),
        });

        let topology = SketchTopology::from_sketch(&sketch);

        assert_eq!(topology.profiles().len(), 2);
        assert_eq!(topology.edges().len(), 7);
    }

    #[test]
    fn topology_finds_profile_by_geometry_independent_of_rotation() {
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

        let topology = SketchTopology::from_sketch(&sketch);
        let rotated = SketchProfile::Polygon {
            points: vec![
                dvec2(10.0, 0.0),
                dvec2(10.0, 8.0),
                dvec2(0.0, 8.0),
                dvec2(0.0, 0.0),
            ],
        };

        assert!(topology.find_profile(&rotated).is_some());
    }
}
