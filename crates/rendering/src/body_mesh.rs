//! Mesh generation for extruded sketch profiles.
//! Produces triangle meshes with per-vertex normals and classified edges
//! for high-quality shaded rendering in the viewport.

use glam::{DVec2, DVec3};
use roncad_geometry::SketchProfile;

const PROFILE_EPSILON: f64 = 1e-6;
const CIRCLE_SEGMENTS_MIN: usize = 24;
const CIRCLE_SEGMENTS_MAX: usize = 96;
const CIRCLE_SEGMENTS_PER_MM: f64 = 1.0;
const CIRCLE_CONNECTORS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct MeshVertex3d {
    pub position: DVec3,
    pub normal: DVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshTriangle3d {
    pub vertices: [MeshVertex3d; 3],
}

impl MeshTriangle3d {
    /// Average face normal from vertex normals.
    pub fn face_normal(&self) -> DVec3 {
        let avg = self.vertices[0].normal + self.vertices[1].normal + self.vertices[2].normal;
        let len = avg.length();
        if len > PROFILE_EPSILON {
            avg / len
        } else {
            geometric_normal(&self.vertices.each_ref().map(|v| v.position))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Hard crease edge (polygon corners, cap boundaries).
    Crease,
    /// Smooth border edge (top/bottom outline of curved sections).
    Border,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshEdge3d {
    pub start: DVec3,
    pub end: DVec3,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtrudeMesh3d {
    pub triangles: Vec<MeshTriangle3d>,
    pub edges: Vec<MeshEdge3d>,
}

// Legacy compat: keep the old name available internally for the outline_edges
// field consumers that destructure. The public API is `edges` now.
impl ExtrudeMesh3d {
    pub fn outline_edge_pairs(&self) -> Vec<(DVec3, DVec3)> {
        self.edges.iter().map(|e| (e.start, e.end)).collect()
    }
}

pub fn extrude_mesh(profile: &SketchProfile, distance_mm: f64) -> ExtrudeMesh3d {
    let (outline_2d, is_smooth_profile) = profile_outline_with_kind(profile);
    if outline_2d.len() < 3 {
        return ExtrudeMesh3d {
            triangles: Vec::new(),
            edges: Vec::new(),
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

    // --- Cap faces (flat shading) ---
    for [a, b, c] in triangulate_polygon(&outline_2d) {
        push_flat_triangle(&mut triangles, [base[a], base[b], base[c]], bottom_normal);
        push_flat_triangle(&mut triangles, [cap[a], cap[b], cap[c]], top_normal);
    }

    // --- Side faces ---
    let signed_area = signed_polygon_area(&outline_2d);
    let n = outline_2d.len();

    // Precompute per-vertex smooth normals for circle profiles
    let smooth_normals: Option<Vec<DVec3>> = if is_smooth_profile {
        Some(
            outline_2d
                .iter()
                .map(|p| {
                    let radial = DVec3::new(p.x - centroid_2d.x, p.y - centroid_2d.y, 0.0);
                    let len = radial.length();
                    if len > PROFILE_EPSILON {
                        if signed_area >= 0.0 {
                            radial / len
                        } else {
                            -radial / len
                        }
                    } else {
                        DVec3::Z
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    for index in 0..n {
        let next = (index + 1) % n;
        let edge = outline_2d[next] - outline_2d[index];
        if edge.length_squared() <= PROFILE_EPSILON * PROFILE_EPSILON {
            continue;
        }

        if let Some(ref normals) = smooth_normals {
            // Smooth shading: each vertex gets its radial normal
            let n0 = normals[index];
            let n1 = normals[next];

            let outward = if signed_area >= 0.0 {
                DVec3::new(edge.y, -edge.x, 0.0)
            } else {
                DVec3::new(-edge.y, edge.x, 0.0)
            };
            push_smooth_quad(
                &mut triangles,
                base[index],
                base[next],
                cap[next],
                cap[index],
                n0,
                n1,
                outward,
            );
        } else {
            // Flat shading: per-face normal from edge direction
            let outward = if signed_area >= 0.0 {
                DVec3::new(edge.y, -edge.x, 0.0)
            } else {
                DVec3::new(-edge.y, edge.x, 0.0)
            };
            push_flat_triangle(
                &mut triangles,
                [base[index], base[next], cap[next]],
                outward,
            );
            push_flat_triangle(
                &mut triangles,
                [base[index], cap[next], cap[index]],
                outward,
            );
        }
    }

    // --- Edges ---
    let mut edges = Vec::new();
    let edge_kind_for_outline = if is_smooth_profile {
        EdgeKind::Border
    } else {
        EdgeKind::Crease
    };

    for index in 0..base.len() {
        let next = (index + 1) % base.len();
        edges.push(MeshEdge3d {
            start: base[index],
            end: base[next],
            kind: edge_kind_for_outline,
        });
        edges.push(MeshEdge3d {
            start: cap[index],
            end: cap[next],
            kind: edge_kind_for_outline,
        });
    }
    for index in connector_indices(profile, base.len()) {
        let kind = if is_smooth_profile {
            EdgeKind::Border
        } else {
            EdgeKind::Crease
        };
        edges.push(MeshEdge3d {
            start: base[index],
            end: cap[index],
            kind,
        });
    }

    // Centroid-based fallback for degenerate polygons
    if triangles.is_empty() {
        for index in 0..outline_2d.len() {
            let next = (index + 1) % outline_2d.len();
            let edge_mid = (outline_2d[index] + outline_2d[next]) * 0.5;
            let outward = DVec3::new(edge_mid.x - centroid_2d.x, edge_mid.y - centroid_2d.y, 0.0);
            push_flat_triangle(
                &mut triangles,
                [base[index], base[next], cap[next]],
                outward,
            );
            push_flat_triangle(
                &mut triangles,
                [base[index], cap[next], cap[index]],
                outward,
            );
        }
    }

    ExtrudeMesh3d { triangles, edges }
}

fn push_flat_triangle(
    triangles: &mut Vec<MeshTriangle3d>,
    positions: [DVec3; 3],
    outward_hint: DVec3,
) {
    let normal = geometric_normal(&positions);
    if normal.length_squared() <= PROFILE_EPSILON * PROFILE_EPSILON {
        return;
    }

    let (positions, normal) = if outward_hint.length_squared() > PROFILE_EPSILON * PROFILE_EPSILON
        && normal.dot(outward_hint) < 0.0
    {
        let swapped = [positions[0], positions[2], positions[1]];
        (swapped, geometric_normal(&swapped).normalize())
    } else {
        (positions, normal.normalize())
    };

    triangles.push(MeshTriangle3d {
        vertices: [
            MeshVertex3d {
                position: positions[0],
                normal,
            },
            MeshVertex3d {
                position: positions[1],
                normal,
            },
            MeshVertex3d {
                position: positions[2],
                normal,
            },
        ],
    });
}

fn push_smooth_quad(
    triangles: &mut Vec<MeshTriangle3d>,
    bl: DVec3,
    br: DVec3,
    tr: DVec3,
    tl: DVec3,
    normal_left: DVec3,
    normal_right: DVec3,
    outward_hint: DVec3,
) {
    let geometric = (br - bl).cross(tr - bl);
    let flip = outward_hint.length_squared() > PROFILE_EPSILON * PROFILE_EPSILON
        && geometric.dot(outward_hint) < 0.0;

    if flip {
        triangles.push(MeshTriangle3d {
            vertices: [
                MeshVertex3d { position: bl, normal: normal_left },
                MeshVertex3d { position: tr, normal: normal_right },
                MeshVertex3d { position: br, normal: normal_right },
            ],
        });
        triangles.push(MeshTriangle3d {
            vertices: [
                MeshVertex3d { position: bl, normal: normal_left },
                MeshVertex3d { position: tl, normal: normal_left },
                MeshVertex3d { position: tr, normal: normal_right },
            ],
        });
    } else {
        triangles.push(MeshTriangle3d {
            vertices: [
                MeshVertex3d { position: bl, normal: normal_left },
                MeshVertex3d { position: br, normal: normal_right },
                MeshVertex3d { position: tr, normal: normal_right },
            ],
        });
        triangles.push(MeshTriangle3d {
            vertices: [
                MeshVertex3d { position: bl, normal: normal_left },
                MeshVertex3d { position: tr, normal: normal_right },
                MeshVertex3d { position: tl, normal: normal_left },
            ],
        });
    }
}

fn geometric_normal(positions: &[DVec3; 3]) -> DVec3 {
    (positions[1] - positions[0]).cross(positions[2] - positions[0])
}

fn circle_segment_count(radius: f64) -> usize {
    let count = (radius * CIRCLE_SEGMENTS_PER_MM) as usize;
    count.clamp(CIRCLE_SEGMENTS_MIN, CIRCLE_SEGMENTS_MAX)
}

fn profile_outline_with_kind(profile: &SketchProfile) -> (Vec<DVec2>, bool) {
    match profile {
        SketchProfile::Polygon { points } => (points.clone(), false),
        SketchProfile::Circle { center, radius } => {
            let segments = circle_segment_count(*radius);
            let points = (0..segments)
                .map(|index| {
                    let angle = std::f64::consts::TAU * index as f64 / segments as f64;
                    *center + DVec2::new(angle.cos(), angle.sin()) * *radius
                })
                .collect();
            (points, true)
        }
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
        let mut best_ear = None;
        let mut best_score = f64::NEG_INFINITY;

        for i in 0..remaining.len() {
            let prev = remaining[(i + remaining.len() - 1) % remaining.len()];
            let curr = remaining[i];
            let next = remaining[(i + 1) % remaining.len()];

            let p_prev = points[prev];
            let p_curr = points[curr];
            let p_next = points[next];

            if !is_convex(p_prev, p_curr, p_next) {
                continue;
            }

            // Check if any other point is inside this ear
            let mut is_ear = true;
            for &candidate in &remaining {
                if candidate != prev && candidate != curr && candidate != next {
                    if point_in_triangle(points[candidate], p_prev, p_curr, p_next) {
                        is_ear = false;
                        break;
                    }
                }
            }

            if is_ear {
                // Score the ear based on its minimum angle (higher is closer to equilateral)
                let v1 = (p_prev - p_curr).normalize_or_zero();
                let v2 = (p_next - p_curr).normalize_or_zero();
                let v3 = (p_next - p_prev).normalize_or_zero();
                
                let dot1 = v1.dot(v2);
                let dot2 = (-v1).dot(v3);
                let dot3 = (-v2).dot(-v3);
                
                // Minimum angle corresponds to the maximum dot product
                let max_dot = dot1.max(dot2).max(dot3);
                // We want to minimize the max_dot (which means maximizing the min angle)
                let score = -max_dot;

                if score > best_score {
                    best_score = score;
                    best_ear = Some(i);
                }
            }
        }

        if let Some(ear_idx) = best_ear {
            let prev = remaining[(ear_idx + remaining.len() - 1) % remaining.len()];
            let curr = remaining[ear_idx];
            let next = remaining[(ear_idx + 1) % remaining.len()];
            triangles.push([prev, curr, next]);
            remaining.remove(ear_idx);
        } else {
            // Fallback: just clip the first convex ear if score fails, or abort if none
            let mut clipped = false;
            for i in 0..remaining.len() {
                let prev = remaining[(i + remaining.len() - 1) % remaining.len()];
                let curr = remaining[i];
                let next = remaining[(i + 1) % remaining.len()];
                if is_convex(points[prev], points[curr], points[next]) {
                    triangles.push([prev, curr, next]);
                    remaining.remove(i);
                    clipped = true;
                    break;
                }
            }
            if !clipped {
                break; // Give up, prevent infinite loop
            }
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
        assert_eq!(mesh.edges.len(), 12);
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
            .all(|triangle| triangle.face_normal().is_finite()));
    }

    #[test]
    fn circle_extrude_has_smooth_radial_normals() {
        let mesh = extrude_mesh(
            &SketchProfile::Circle {
                center: dvec2(0.0, 0.0),
                radius: 10.0,
            },
            5.0,
        );

        assert!(!mesh.triangles.is_empty());

        // Check that side-face vertex normals are roughly radial (z component ≈ 0)
        let side_triangles: Vec<_> = mesh
            .triangles
            .iter()
            .filter(|tri| {
                let fn_ = tri.face_normal();
                fn_.z.abs() < 0.5 // not a cap face
            })
            .collect();
        assert!(!side_triangles.is_empty());

        for tri in &side_triangles {
            for v in &tri.vertices {
                assert!(
                    v.normal.z.abs() < 0.01,
                    "side vertex normal should have near-zero z, got {:?}",
                    v.normal
                );
                assert!(
                    v.normal.length() > 0.99,
                    "vertex normal should be unit length"
                );
            }
        }
    }

    #[test]
    fn polygon_edges_are_crease_kind() {
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

        assert!(mesh
            .edges
            .iter()
            .all(|e| e.kind == super::EdgeKind::Crease));
    }

    #[test]
    fn circle_edges_are_border_kind() {
        let mesh = extrude_mesh(
            &SketchProfile::Circle {
                center: dvec2(0.0, 0.0),
                radius: 5.0,
            },
            10.0,
        );

        assert!(mesh
            .edges
            .iter()
            .all(|e| e.kind == super::EdgeKind::Border));
    }
}
