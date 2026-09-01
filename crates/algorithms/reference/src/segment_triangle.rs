//! Certified segment/triangle topology classification.
//!
//! This module decides topology without a caller epsilon. Metric witness
//! construction is deliberately separate: a representable closest point is not
//! a certified proof of intersection.

use axiolid_contracts::Sign;
use axiolid_core::Point3;

use crate::orient3d;

/// The topological relation between a finite segment and one triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentTriangleRelation {
    /// The closed primitives have no common point.
    Disjoint,
    /// The segment crosses the triangle interior at one point.
    Proper,
    /// The primitives meet at an endpoint, edge, or vertex.
    Touching,
    /// Both segment endpoints lie in the triangle plane.
    ///
    /// This deliberately does not claim overlap: callers needing coplanar
    /// coverage use a projected 2D predicate with an explicit policy.
    Coplanar,
    /// The segment endpoints are identical.
    DegenerateSegment,
    /// The three triangle vertices are collinear.
    DegenerateTriangle,
}

/// Classify a segment against a triangle using certified orientation signs.
///
/// No tolerance is accepted: this reports the exact topology of the supplied
/// binary64 coordinates. Nearness is a metric/policy question for the caller.
#[must_use]
pub fn segment_triangle_relation(
    start: Point3,
    end: Point3,
    triangle: [Point3; 3],
) -> SegmentTriangleRelation {
    if start == end {
        return SegmentTriangleRelation::DegenerateSegment;
    }

    let [a, b, c] = triangle;
    if (b - a).cross(c - a).length_squared() == 0.0 {
        return SegmentTriangleRelation::DegenerateTriangle;
    }

    let start_side = sign(orient3d(a, b, c, start));
    let end_side = sign(orient3d(a, b, c, end));
    if start_side == Sign::Zero && end_side == Sign::Zero {
        return SegmentTriangleRelation::Coplanar;
    }
    if start_side != Sign::Zero && start_side == end_side {
        return SegmentTriangleRelation::Disjoint;
    }

    // These three oriented tetrahedra are the barycentric side tests of the
    // segment's plane crossing. They are exact even when the crossing point is
    // not representable as binary64.
    let edge_signs = [
        sign(orient3d(start, end, a, b)),
        sign(orient3d(start, end, b, c)),
        sign(orient3d(start, end, c, a)),
    ];
    let all_nonnegative = edge_signs.iter().all(|&value| value != Sign::Negative);
    let all_nonpositive = edge_signs.iter().all(|&value| value != Sign::Positive);
    if !all_nonnegative && !all_nonpositive {
        return SegmentTriangleRelation::Disjoint;
    }

    if start_side == Sign::Zero || end_side == Sign::Zero || edge_signs.contains(&Sign::Zero) {
        SegmentTriangleRelation::Touching
    } else {
        SegmentTriangleRelation::Proper
    }
}

fn sign(value: axiolid_contracts::Certified) -> Sign {
    // `orient3d` always escalates to an exact, certain sign.
    value
        .sign()
        .expect("orient3d is a total certified predicate")
}
