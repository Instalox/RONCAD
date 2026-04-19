use glam::{DVec2, DVec3};
use roncad_geometry::SketchProfile;

const PROFILE_EPSILON: f64 = 1e-6;
const CIRCLE_SEGMENTS: usize = 48;
const CIRCLE_CONNECTORS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshTriangle3d {
    pub positions: [DVec3; 3],
    pub normal: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeMesh3d {
    pub triangles: Vec<MeshTriangle3d>,
    pub outline_edges: Vec<(DVec3, DVec3)>,
}

pub fn extrude_mesh(profile: &SketchProfile, distance_mm: f64) -> ExtrudeMesh3d {
    let outline_2d = profile_outline_points(profile);
    if outline_2d.len() < 3 {
        return ExtrudeMesh3d {
            triangles: Vec::new(),
            outline_edges: Vec::new(),
        };
    }

    let base: Vec<_> = outline_2d
        .iter()
        .map(|point| DVec3::new(point.x, point.y, 0.0))
        .collect();
    let cap: Vec<_> = outline_2d
        .iter()
        .map(|point| DVec3::new(point.x, point.y, distance_mm))
        .collect();
    let centroid_2d = polygon_centroid(&outline_2d);
    let top_normal = if distance_mm >= 0.0 {
        DVec3::Z
    } else {
        -DVec3::Z
    };
    let bottom_normal = -top_normal;

    let mut triangles = Vec::new();
    for [a, b, c] in triangulate_polygon(&outline_2d) {
        push_oriented_triangle(&mut triangles, [base[a], base[b], base[c]], bottom_normal);
        push_oriented_triangle(&mut triangles, [cap[a], cap[b], cap[c]], top_normal);
    }

    let signed_area = signed_polygon_area(&outline_2d);
    for index in 0..outline_2d.len() {
        let next = (index + 1) % outline_2d.len();
        let edge = outline_2d[next] - outline_2d[index];
        if edge.length_squared() <= PROFILE_EPSILON * PROFILE_EPSILON {
            continue;
        }
        let outward = if signed_area >= 0.0 {
            DVec3::new(edge.y, -edge.x, 0.0)
        } else {
            DVec3::new(-edge.y, edge.x, 0.0)
        };
        push_oriented_triangle(
            &mut triangles,
            [base[index], base[next], cap[next]],
            outward,
        );
        push_oriented_triangle(
            &mut triangles,
            [base[index], cap[next], cap[index]],
            outward,
        );
    }

    let mut outline_edges = Vec::new();
    for index in 0..base.len() {
        let next = (index + 1) % base.len();
        outline_edges.push((base[index], base[next]));
        outline_edges.push((cap[index], cap[next]));
    }
    for index in connector_indices(profile, base.len()) {
        outline_edges.push((base[index], cap[index]));
    }

    // If the polygon is numerically tiny or oddly shaped, fall back to
    // centroid-based side hints so rendering still has a stable mesh.
    if triangles.is_empty() {
        for index in 0..outline_2d.len() {
            let next = (index + 1) % outline_2d.len();
            let edge_mid = (outline_2d[index] + outline_2d[next]) * 0.5;
            let outward = DVec3::new(edge_mid.x - centroid_2d.x, edge_mid.y - centroid_2d.y, 0.0);
            push_oriented_triangle(
                &mut triangles,
                [base[index], base[next], cap[next]],
                outward,
            );
            push_oriented_triangle(
                &mut triangles,
                [base[index], cap[next], cap[index]],
                outward,
            );
        }
    }

    ExtrudeMesh3d {
        triangles,
        outline_edges,
    }
}

fn push_oriented_triangle(
    triangles: &mut Vec<MeshTriangle3d>,
    positions: [DVec3; 3],
    outward_hint: DVec3,
) {
    let normal = triangle_normal(positions);
    if normal.length_squared() <= PROFILE_EPSILON * PROFILE_EPSILON {
        return;
    }

    let (positions, normal) = if outward_hint.length_squared() > PROFILE_EPSILON * PROFILE_EPSILON
        && normal.dot(outward_hint) < 0.0
    {
        let swapped = [positions[0], positions[2], positions[1]];
        (swapped, triangle_normal(swapped).normalize())
    } else {
        (positions, normal.normalize())
    };

    triangles.push(MeshTriangle3d { positions, normal });
}

fn triangle_normal(positions: [DVec3; 3]) -> DVec3 {
    (positions[1] - positions[0]).cross(positions[2] - positions[0])
}

fn profile_outline_points(profile: &SketchProfile) -> Vec<DVec2> {
    match profile {
        SketchProfile::Polygon { points } => points.clone(),
        SketchProfile::Circle { center, radius } => (0..CIRCLE_SEGMENTS)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / CIRCLE_SEGMENTS as f64;
                *center + DVec2::new(angle.cos(), angle.sin()) * *radius
            })
            .collect(),
    }
}

fn connector_indices(profile: &SketchProfile, point_count: usize) -> Vec<usize> {
    match profile {
        SketchProfile::Polygon { .. } if point_count <= 12 => (0..point_count).collect(),
        SketchProfile::Polygon { .. } | SketchProfile::Circle { .. } => {
            let step = (point_count / CIRCLE_CONNECTORS).max(1);
            let mut indices: Vec<_> = (0..point_count).step_by(step).collect();
            indices.truncate(CIRCLE_CONNECTORS);
            indices
        }
    }
}

fn triangulate_polygon(points: &[DVec2]) -> Vec<[usize; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }

    let mut remaining: Vec<usize> = if signed_polygon_area(points) >= 0.0 {
        (0..points.len()).collect()
    } else {
        (0..points.len()).rev().collect()
    };

    let mut triangles = Vec::new();
    while remaining.len() > 3 {
        let mut clipped = false;
        for i in 0..remaining.len() {
            let prev = remaining[(i + remaining.len() - 1) % remaining.len()];
            let curr = remaining[i];
            let next = remaining[(i + 1) % remaining.len()];

            if !is_convex(points[prev], points[curr], points[next]) {
                continue;
            }

            if remaining.iter().copied().any(|candidate| {
                candidate != prev
                    && candidate != curr
                    && candidate != next
                    && point_in_triangle(
                        points[candidate],
                        points[prev],
                        points[curr],
                        points[next],
                    )
            }) {
                continue;
            }

            triangles.push([prev, curr, next]);
            remaining.remove(i);
            clipped = true;
            break;
        }

        if !clipped {
            return Vec::new();
        }
    }

    if remaining.len() == 3 {
        triangles.push([remaining[0], remaining[1], remaining[2]]);
    }

    triangles
}

fn is_convex(a: DVec2, b: DVec2, c: DVec2) -> bool {
    (b - a).perp_dot(c - b) > PROFILE_EPSILON
}

fn point_in_triangle(p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
    let ab = (b - a).perp_dot(p - a);
    let bc = (c - b).perp_dot(p - b);
    let ca = (a - c).perp_dot(p - c);

    (ab >= -PROFILE_EPSILON && bc >= -PROFILE_EPSILON && ca >= -PROFILE_EPSILON)
        || (ab <= PROFILE_EPSILON && bc <= PROFILE_EPSILON && ca <= PROFILE_EPSILON)
}

fn signed_polygon_area(points: &[DVec2]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        area += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    area * 0.5
}

fn polygon_centroid(points: &[DVec2]) -> DVec2 {
    let area = signed_polygon_area(points);
    if area.abs() <= PROFILE_EPSILON {
        let sum = points
            .iter()
            .copied()
            .fold(DVec2::ZERO, |acc, point| acc + point);
        return sum / points.len() as f64;
    }

    let mut centroid = DVec2::ZERO;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let cross = points[index].x * points[next].y - points[next].x * points[index].y;
        centroid += (points[index] + points[next]) * cross;
    }
    centroid / (6.0 * area)
}

#[cfg(test)]
mod tests {
    use glam::dvec2;
    use roncad_geometry::SketchProfile;

    use super::extrude_mesh;

    #[test]
    fn rectangle_extrude_builds_caps_and_sides() {
        let mesh = extrude_mesh(
            &SketchProfile::Polygon {
                points: vec![
                    dvec2(0.0, 0.0),
                    dvec2(10.0, 0.0),
                    dvec2(10.0, 5.0),
                    dvec2(0.0, 5.0),
                ],
            },
            12.0,
        );

        assert_eq!(mesh.triangles.len(), 12);
        assert_eq!(mesh.outline_edges.len(), 12);
    }

    #[test]
    fn concave_polygon_still_tessellates() {
        let mesh = extrude_mesh(
            &SketchProfile::Polygon {
                points: vec![
                    dvec2(0.0, 0.0),
                    dvec2(8.0, 0.0),
                    dvec2(8.0, 2.0),
                    dvec2(4.0, 2.0),
                    dvec2(4.0, 6.0),
                    dvec2(0.0, 6.0),
                ],
            },
            5.0,
        );

        assert!(!mesh.triangles.is_empty());
        assert!(mesh
            .triangles
            .iter()
            .all(|triangle| triangle.normal.is_finite()));
    }
}
