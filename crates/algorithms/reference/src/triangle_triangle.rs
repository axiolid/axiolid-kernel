//! Certified triangle/triangle topology classification.
//!
//! Coplanar triangle overlap is deliberately reported as [`TriangleTriangleRelation::Coplanar`]
//! rather than collapsed into contact. It requires a caller to make any 2D surface policy explicit.

use axiolid_core::{Point2, Point3};
use axiolid_kernel::Sign;

use crate::{orient2d, segment_triangle_relation, SegmentTriangleRelation};

/// The exact topological relation between two finite triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriangleTriangleRelation {
    /// The closed triangle primitives have no common point.
    Disjoint,
    /// The triangles cross transversally through both interiors.
    Proper,
    /// The triangles meet at a vertex, edge, or other non-transverse contact.
    Touching,
    /// All vertices lie in one plane. This does not assert planar overlap.
    Coplanar,
    /// At least one triangle has collinear vertices.
    DegenerateTriangle,
}

/// Classify two triangles using certified orientation predicates.
///
/// This accepts no tolerance: it reports topology for the supplied binary64
/// coordinates. Metric overlap lengths and coplanar-surface policy belong in
/// higher layers.
#[must_use]
pub fn triangle_triangle_relation(
    first: [Point3; 3],
    second: [Point3; 3],
) -> TriangleTriangleRelation {
    if degenerate(first) || degenerate(second) {
        return TriangleTriangleRelation::DegenerateTriangle;
    }

    let second_in_first_plane =
        second.map(|point| sign(crate::orient3d(first[0], first[1], first[2], point)));
    if second_in_first_plane.iter().all(|&side| side == Sign::Zero) {
        return TriangleTriangleRelation::Coplanar;
    }

    let mut touching = false;
    for triangle in [first, second] {
        let other = if triangle == first { second } else { first };
        for edge in [
            [triangle[0], triangle[1]],
            [triangle[1], triangle[2]],
            [triangle[2], triangle[0]],
        ] {
            match segment_triangle_relation(edge[0], edge[1], other) {
                SegmentTriangleRelation::Proper => return TriangleTriangleRelation::Proper,
                SegmentTriangleRelation::Touching => touching = true,
                SegmentTriangleRelation::Coplanar => {
                    touching |= coplanar_segment_touches_triangle(edge, other);
                }
                SegmentTriangleRelation::Disjoint => {}
                SegmentTriangleRelation::DegenerateSegment
                | SegmentTriangleRelation::DegenerateTriangle => {
                    unreachable!("validated triangles have non-degenerate edges")
                }
            }
        }
    }

    if touching {
        TriangleTriangleRelation::Touching
    } else {
        TriangleTriangleRelation::Disjoint
    }
}

fn degenerate([a, b, c]: [Point3; 3]) -> bool {
    (b - a).cross(c - a).length_squared() == 0.0
}

fn coplanar_segment_touches_triangle(segment: [Point3; 2], triangle: [Point3; 3]) -> bool {
    let normal = (triangle[1] - triangle[0]).cross(triangle[2] - triangle[0]);
    let axis = dominant_axis(normal);
    let [start, end] = segment.map(|point| project(point, axis));
    let projected = triangle.map(|point| project(point, axis));

    point_in_triangle(start, projected)
        || point_in_triangle(end, projected)
        || [
            [projected[0], projected[1]],
            [projected[1], projected[2]],
            [projected[2], projected[0]],
        ]
        .into_iter()
        .any(|edge| segments_touch([start, end], edge))
}

fn point_in_triangle(point: Point2, [a, b, c]: [Point2; 3]) -> bool {
    let signs = [
        sign(orient2d(a, b, point)),
        sign(orient2d(b, c, point)),
        sign(orient2d(c, a, point)),
    ];
    signs.iter().all(|&side| side != Sign::Negative)
        || signs.iter().all(|&side| side != Sign::Positive)
}

fn segments_touch([a, b]: [Point2; 2], [c, d]: [Point2; 2]) -> bool {
    let ab_c = sign(orient2d(a, b, c));
    let ab_d = sign(orient2d(a, b, d));
    let cd_a = sign(orient2d(c, d, a));
    let cd_b = sign(orient2d(c, d, b));
    straddles(ab_c, ab_d) && straddles(cd_a, cd_b)
}

fn straddles(first: Sign, second: Sign) -> bool {
    first == Sign::Zero || second == Sign::Zero || first != second
}

fn dominant_axis(vector: Point3) -> usize {
    let absolute = vector.abs();
    if absolute.x >= absolute.y && absolute.x >= absolute.z {
        0
    } else if absolute.y >= absolute.z {
        1
    } else {
        2
    }
}

fn project(point: Point3, axis: usize) -> Point2 {
    match axis {
        0 => Point2::new(point.y, point.z),
        1 => Point2::new(point.x, point.z),
        _ => Point2::new(point.x, point.y),
    }
}

fn sign(value: axiolid_kernel::Certified) -> Sign {
    value.sign().expect("certified predicates are total")
}
