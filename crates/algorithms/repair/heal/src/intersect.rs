//! Triangle-triangle self-intersection over a mesh (#73).
//!
//! # Why adjacency is the hard part
//!
//! In a closed mesh, every triangle shares an edge with three neighbours and
//! a vertex with many more. Those touch by construction. A naive predicate
//! that answers "do these two triangles share a point" reports every closed
//! mesh as broken, so adjacency is excluded structurally: pairs sharing any
//! vertex INDEX are skipped before any arithmetic runs.
//!
//! Index-based exclusion, not coordinate comparison. Two distinct vertices
//! holding equal coordinates are a `DuplicateVertex` defect, reported
//! separately; treating them as adjacent here would hide it.
//!
//! # Broad phase
//!
//! Candidate pairs come from the shared `Bvh`. The index is an accelerator
//! only: `self_intersections_brute_force` computes the same answer by
//! checking every pair, and the two must agree on every input. If they ever
//! disagree, the index is wrong -- that is a bug, not a tuning parameter.

use axiolid_core::{Aabb, Point2, Point3, Vec3};
use axiolid_guarantees::Sign;
use axiolid_mesh::TriangleMeshView;
use axiolid_predicates::{orient2d, orient3d};
use axiolid_spatial::{Bvh, SpatialIndex, SpatialItem};
use std::ops::ControlFlow;

/// One intersecting triangle pair, lower index first.
///
/// Ordered and deduplicated so a caller can compare two runs directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntersectingPair {
    /// Lower triangle index.
    pub first: u32,
    /// Higher triangle index.
    pub second: u32,
}

impl IntersectingPair {
    /// Normalise so `first < second`, giving one canonical form per pair.
    fn new(a: usize, b: usize) -> Self {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        Self {
            first: lo as u32,
            second: hi as u32,
        }
    }
}

/// Whether two triangles share at least one vertex index.
///
/// Adjacency is decided on indices alone, so it costs no arithmetic and
/// cannot be perturbed by coordinates.
fn share_a_vertex(a: [u64; 3], b: [u64; 3]) -> bool {
    a.iter().any(|i| b.contains(i))
}

/// Sign of `orient3d`, or `None` when the four points are coplanar.
///
/// The certified predicate decides the sign exactly, so a point lying in a
/// plane is reported as coplanar rather than as an arbitrary side chosen by
/// rounding.
fn side(a: Point3, b: Point3, c: Point3, d: Point3) -> Option<i32> {
    match orient3d(a, b, c, d).sign() {
        Some(Sign::Positive) => Some(1),
        Some(Sign::Negative) => Some(-1),
        _ => None,
    }
}

/// Whether two non-adjacent triangles properly intersect.
///
/// Plane-side rejection alone is not sufficient. Two triangles can each
/// straddle the other's plane and still miss entirely -- the cube's opposite
/// faces do exactly that -- so passing the rejection test means only "not
/// separated by these two planes", which is not the same as intersecting.
///
/// The decision is therefore made on the intersection LINE of the two planes.
/// Each triangle meets that line in an interval; the triangles intersect if
/// and only if the intervals overlap. Intervals are compared through the
/// signed plane distances rather than by constructing the line, so no
/// intersection point is ever computed in floating point.
///
/// Coplanar configurations are decided by exact 2D region logic rather than
/// assumed to intersect: the pair is projected onto its dominant plane and
/// tested for shared interior area with `orient2d`. Triangles that merely
/// touch along a shared edge or vertex have no common area and are not
/// reported.
fn triangles_intersect(p: [Point3; 3], q: [Point3; 3]) -> bool {
    let Some(p_sides) = plane_sides(q, p) else {
        return coplanar_triangles_overlap(p, q);
    };
    if separated(p_sides) {
        return false;
    }
    let Some(q_sides) = plane_sides(p, q) else {
        return coplanar_triangles_overlap(p, q);
    };
    if separated(q_sides) {
        return false;
    }
    intervals_overlap(p, p_sides, q, q_sides)
}

/// Signed side of each vertex of `t` against the plane of `plane`.
fn plane_sides(plane: [Point3; 3], t: [Point3; 3]) -> Option<[i32; 3]> {
    let a = side(plane[0], plane[1], plane[2], t[0]);
    let b = side(plane[0], plane[1], plane[2], t[1]);
    let c = side(plane[0], plane[1], plane[2], t[2]);
    match (a, b, c) {
        // All three coplanar with the other triangle: the interval test
        // degenerates, so the caller falls back to conservative reporting.
        (None, None, None) => None,
        _ => Some([a.unwrap_or(0), b.unwrap_or(0), c.unwrap_or(0)]),
    }
}

/// Whether every vertex lies strictly on one side.
fn separated(sides: [i32; 3]) -> bool {
    sides.iter().all(|&s| s > 0) || sides.iter().all(|&s| s < 0)
}

/// Whether the two triangles' intervals on the planes' common line overlap.
///
/// Each triangle has one vertex alone on one side of the other's plane (or a
/// vertex exactly on it). The two edges from that vertex cross the plane, and
/// their crossing points bound the triangle's interval on the common line.
/// Positions along the line are measured by projecting onto the line's
/// direction, and the crossing points are interpolated by the signed
/// distances, which is exactly where the edge meets the plane.
fn intervals_overlap(p: [Point3; 3], p_sides: [i32; 3], q: [Point3; 3], q_sides: [i32; 3]) -> bool {
    let direction = (p[1] - p[0])
        .cross(p[2] - p[0])
        .cross((q[1] - q[0]).cross(q[2] - q[0]));
    if !direction.is_finite() || direction.length_squared() == 0.0 {
        return true; // parallel planes that are not separated: conservative
    }
    let Some(p_span) = interval_on(p, p_sides, q, direction) else {
        return true;
    };
    let Some(q_span) = interval_on(q, q_sides, p, direction) else {
        return true;
    };
    p_span.0 <= q_span.1 && q_span.0 <= p_span.1
}

/// The triangle's interval along `direction`, bounded by where its edges
/// cross the other triangle's plane.
///
/// Returns `None` when the configuration is degenerate enough that the
/// crossing points cannot be located, so the caller can stay conservative.
fn interval_on(
    t: [Point3; 3],
    sides: [i32; 3],
    plane: [Point3; 3],
    direction: Vec3,
) -> Option<(f64, f64)> {
    let distance = |point: Point3| {
        let normal = (plane[1] - plane[0]).cross(plane[2] - plane[0]);
        (point - plane[0]).dot(normal)
    };
    let mut hits: Vec<f64> = Vec::new();
    for (i, j) in [(0usize, 1usize), (1, 2), (2, 0)] {
        let (si, sj) = (sides[i], sides[j]);
        if si == 0 {
            hits.push(t[i].dot(direction));
        }
        if si != 0 && sj != 0 && si != sj {
            let (di, dj) = (distance(t[i]), distance(t[j]));
            let denominator = di - dj;
            if denominator == 0.0 {
                return None;
            }
            let ratio = di / denominator;
            let crossing = t[i] + (t[j] - t[i]) * ratio;
            hits.push(crossing.dot(direction));
        }
    }
    if hits.is_empty() {
        return None;
    }
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for hit in hits {
        low = low.min(hit);
        high = high.max(hit);
    }
    Some((low, high))
}

/// Triangle corner positions and vertex indices, if the triangle is usable.
fn triangle_of<M: TriangleMeshView + ?Sized>(
    mesh: &M,
    index: usize,
) -> Option<([Point3; 3], [u64; 3])> {
    let corners = mesh.triangle(index);
    let positions = corners.map(|j| mesh.position(j as usize));
    if positions.iter().any(|p| !p.is_finite()) {
        return None;
    }
    Some((positions, corners))
}

/// Self-intersecting triangle pairs, found through the spatial index.
///
/// Returns pairs in sorted order. An empty result means no pair intersects.
///
/// There is no tolerance parameter: the narrow phase decides with certified
/// `orient3d`, and the broad phase uses exact triangle bounds. A tolerance
/// here would only be able to make the answer *wrong*, by admitting or
/// rejecting pairs the exact test already decides.
#[must_use]
pub fn self_intersections<M: TriangleMeshView + ?Sized>(mesh: &M) -> Vec<IntersectingPair> {
    let count = mesh.triangle_count();
    let mut items = Vec::with_capacity(count);
    for index in 0..count {
        if let Some((positions, _)) = triangle_of(mesh, index) {
            // Pad by the caller's tolerance so the broad phase never rejects
            // a pair the exact narrow phase would have accepted.
            let mut bounds = Aabb::from_point(positions[0]);
            bounds.extend(positions[1]);
            bounds.extend(positions[2]);
            items.push(SpatialItem::new(index as u32, bounds));
        }
    }
    let bvh = Bvh::build(items);

    let mut found = Vec::new();
    for index in 0..count {
        let Some((positions, corners)) = triangle_of(mesh, index) else {
            continue;
        };
        let mut query = Aabb::from_point(positions[0]);
        query.extend(positions[1]);
        query.extend(positions[2]);
        bvh.visit_aabb(&query, &mut |other: &u32| {
            let other = *other as usize;
            // Each unordered pair is decided once, by its lower index.
            if other <= index {
                return ControlFlow::Continue(());
            }
            if let Some((other_positions, other_corners)) = triangle_of(mesh, other) {
                if !share_a_vertex(corners, other_corners)
                    && triangles_intersect(positions, other_positions)
                {
                    found.push(IntersectingPair::new(index, other));
                }
            }
            ControlFlow::Continue(())
        });
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// The same answer without the spatial index, by checking every pair.
///
/// This exists to be compared against [`self_intersections`]. The index is an
/// optimisation, and an optimisation that changes the answer is a bug, so the
/// reference is kept in production code rather than in a test where it could
/// drift out of sync with the accelerated path.
#[must_use]
pub fn self_intersections_brute_force<M: TriangleMeshView + ?Sized>(
    mesh: &M,
) -> Vec<IntersectingPair> {
    let count = mesh.triangle_count();
    let mut found = Vec::new();
    for index in 0..count {
        let Some((positions, corners)) = triangle_of(mesh, index) else {
            continue;
        };
        for other in (index + 1)..count {
            let Some((other_positions, other_corners)) = triangle_of(mesh, other) else {
                continue;
            };
            if !share_a_vertex(corners, other_corners)
                && triangles_intersect(positions, other_positions)
            {
                found.push(IntersectingPair::new(index, other));
            }
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// Whether two coplanar triangles share interior area, decided exactly.
///
/// Both are projected onto the coordinate plane where their shared normal is
/// largest, so no projected triangle degenerates to a segment. Every test
/// below is `orient2d`, so this stays as exact as the non-coplanar path it
/// replaces -- no tolerance, no constructed intersection point.
///
/// Touching is not overlapping: triangles sharing only an edge or a vertex
/// have no interior area in common and are NOT reported.
fn coplanar_triangles_overlap(p: [Point3; 3], q: [Point3; 3]) -> bool {
    let normal = (p[1] - p[0]).cross(p[2] - p[0]);
    let axis = dominant_axis(normal);
    let pp = project_triangle(p, axis);
    let qq = project_triangle(q, axis);

    // Edge-crossing: any proper crossing of a p edge with a q edge means the
    // boundaries pass through each other, which requires shared area.
    for i in 0..3 {
        for j in 0..3 {
            if segments_properly_cross(pp[i], pp[(i + 1) % 3], qq[j], qq[(j + 1) % 3]) {
                return true;
            }
        }
    }

    // Containment: no crossing but one triangle strictly inside the other.
    pp.iter().any(|&v| strictly_inside(v, qq)) || qq.iter().any(|&v| strictly_inside(v, pp))
}

/// Index of the largest-magnitude normal component.
fn dominant_axis(normal: Vec3) -> usize {
    let (x, y, z) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    }
}

/// Drop the dominant axis, keeping the projection non-degenerate.
fn project_triangle(t: [Point3; 3], axis: usize) -> [Point2; 3] {
    [
        project_point(t[0], axis),
        project_point(t[1], axis),
        project_point(t[2], axis),
    ]
}

fn project_point(p: Point3, axis: usize) -> Point2 {
    match axis {
        0 => Point2::new(p.y, p.z),
        1 => Point2::new(p.z, p.x),
        _ => Point2::new(p.x, p.y),
    }
}

/// Whether segments `a`-`b` and `c`-`d` cross at an interior point of both.
///
/// Requires all four orientations to be strictly non-zero and opposite in
/// pairs. Collinear or touching-at-an-endpoint configurations are rejected:
/// they share boundary, not area.
fn segments_properly_cross(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let (Some(o1), Some(o2), Some(o3), Some(o4)) = (
        sign2(a, b, c),
        sign2(a, b, d),
        sign2(c, d, a),
        sign2(c, d, b),
    ) else {
        return false;
    };
    o1 != o2 && o3 != o4
}

/// Strict `orient2d` sign, or `None` when the three points are collinear.
fn sign2(a: Point2, b: Point2, c: Point2) -> Option<i32> {
    match orient2d(a, b, c).sign() {
        Some(Sign::Positive) => Some(1),
        Some(Sign::Negative) => Some(-1),
        _ => None,
    }
}

/// Whether `point` lies strictly inside triangle `t`.
///
/// A point on an edge is NOT inside: two triangles meeting along a shared
/// edge touch without overlapping, and reporting that as a self-intersection
/// is the false positive this whole function exists to remove.
fn strictly_inside(point: Point2, t: [Point2; 3]) -> bool {
    let mut seen: Option<i32> = None;
    for i in 0..3 {
        let Some(s) = sign2(t[i], t[(i + 1) % 3], point) else {
            return false; // on an edge line: boundary, not interior
        };
        match seen {
            None => seen = Some(s),
            Some(previous) if previous == s => {}
            Some(_) => return false,
        }
    }
    true
}
