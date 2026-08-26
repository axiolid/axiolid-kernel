//! Deterministic metric proximity queries for 3D primitives.
//!
//! These routines construct floating-point witnesses only. Certified topological
//! classification belongs in `axiolid-scalar`; callers that need an exact
//! contact decision must perform that classification separately.

use core::fmt;

use axiolid_core::Point3;

/// Two witnesses, ordered to match the first and second inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPoints3 {
    /// Witness on the first input.
    pub point_a: Point3,
    /// Witness on the second input.
    pub point_b: Point3,
    /// Squared Euclidean separation of the witnesses.
    pub distance_squared: f64,
}

impl ClosestPoints3 {
    fn new(point_a: Point3, point_b: Point3) -> Self {
        Self {
            point_a,
            point_b,
            distance_squared: (point_a - point_b).length_squared(),
        }
    }
}

/// Metric-input failure for primitive proximity queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProximityError {
    /// At least one coordinate is NaN or infinite.
    NonFiniteInput,
    /// A triangle's three vertices are collinear or repeated.
    DegenerateTriangle,
}

impl fmt::Display for ProximityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => formatter.write_str("proximity input must be finite"),
            Self::DegenerateTriangle => formatter.write_str("triangle has zero area"),
        }
    }
}

impl std::error::Error for ProximityError {}

/// Closest witnesses on two finite segments.
///
/// Zero-length segments are intentionally treated as points. Equal-distance
/// solutions preserve the first endpoint of each input segment.
pub fn closest_points_on_segments(
    first: [Point3; 2],
    second: [Point3; 2],
) -> Result<ClosestPoints3, ProximityError> {
    if !first.into_iter().chain(second).all(Point3::is_finite) {
        return Err(ProximityError::NonFiniteInput);
    }

    let [p1, q1] = first;
    let [p2, q2] = second;
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let first_length_squared = d1.dot(d1);
    let second_length_squared = d2.dot(d2);
    let second_projection = d2.dot(r);

    let (mut first_parameter, mut second_parameter);
    if first_length_squared == 0.0 && second_length_squared == 0.0 {
        return Ok(ClosestPoints3::new(p1, p2));
    }
    if first_length_squared == 0.0 {
        first_parameter = 0.0;
        second_parameter = (second_projection / second_length_squared).clamp(0.0, 1.0);
    } else {
        let first_projection = d1.dot(r);
        if second_length_squared == 0.0 {
            second_parameter = 0.0;
            first_parameter = (-first_projection / first_length_squared).clamp(0.0, 1.0);
        } else {
            let directions_dot = d1.dot(d2);
            let denominator =
                first_length_squared * second_length_squared - directions_dot * directions_dot;
            first_parameter = if denominator != 0.0 {
                ((directions_dot * second_projection - first_projection * second_length_squared)
                    / denominator)
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            second_parameter =
                (directions_dot * first_parameter + second_projection) / second_length_squared;
            if second_parameter < 0.0 {
                second_parameter = 0.0;
                first_parameter = (-first_projection / first_length_squared).clamp(0.0, 1.0);
            } else if second_parameter > 1.0 {
                second_parameter = 1.0;
                first_parameter =
                    ((directions_dot - first_projection) / first_length_squared).clamp(0.0, 1.0);
            }
        }
    }

    Ok(ClosestPoints3::new(
        p1 + d1 * first_parameter,
        p2 + d2 * second_parameter,
    ))
}

/// Closest point on a non-degenerate triangle.
pub fn closest_point_on_triangle(
    point: Point3,
    triangle: [Point3; 3],
) -> Result<Point3, ProximityError> {
    validate_triangle(triangle)?;
    if !point.is_finite() {
        return Err(ProximityError::NonFiniteInput);
    }
    Ok(closest_point_on_valid_triangle(point, triangle))
}

/// Deterministic metric witnesses on two non-degenerate triangles.
///
/// Candidate ties retain this stable order: first-triangle vertices,
/// second-triangle vertices, then first/second edge pairs lexicographically.
/// This function does not certify intersection; pair it with scalar predicates
/// when zero separation changes control flow.
pub fn closest_points_on_triangles(
    first: [Point3; 3],
    second: [Point3; 3],
) -> Result<ClosestPoints3, ProximityError> {
    validate_triangle(first)?;
    validate_triangle(second)?;

    let mut best = ClosestPoints3 {
        point_a: Point3::ZERO,
        point_b: Point3::ZERO,
        distance_squared: f64::INFINITY,
    };
    for point in first {
        update_best(
            &mut best,
            ClosestPoints3::new(point, closest_point_on_valid_triangle(point, second)),
        );
    }
    for point in second {
        update_best(
            &mut best,
            ClosestPoints3::new(closest_point_on_valid_triangle(point, first), point),
        );
    }
    for first_edge in triangle_edges(first) {
        for second_edge in triangle_edges(second) {
            update_best(
                &mut best,
                closest_points_on_segments(first_edge, second_edge)?,
            );
        }
    }
    Ok(best)
}

fn validate_triangle(triangle: [Point3; 3]) -> Result<(), ProximityError> {
    if !triangle.into_iter().all(Point3::is_finite) {
        return Err(ProximityError::NonFiniteInput);
    }
    let [a, b, c] = triangle;
    ((b - a).cross(c - a).length_squared() != 0.0)
        .then_some(())
        .ok_or(ProximityError::DegenerateTriangle)
}

fn update_best(best: &mut ClosestPoints3, candidate: ClosestPoints3) {
    if candidate.distance_squared < best.distance_squared {
        *best = candidate;
    }
}

fn triangle_edges(triangle: [Point3; 3]) -> [[Point3; 2]; 3] {
    [
        [triangle[0], triangle[1]],
        [triangle[1], triangle[2]],
        [triangle[2], triangle[0]],
    ]
}

fn closest_point_on_valid_triangle(point: Point3, triangle: [Point3; 3]) -> Point3 {
    let [a, b, c] = triangle;
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let dot_ab_ap = ab.dot(ap);
    let dot_ac_ap = ac.dot(ap);
    if dot_ab_ap <= 0.0 && dot_ac_ap <= 0.0 {
        return a;
    }

    let bp = point - b;
    let dot_ab_bp = ab.dot(bp);
    let dot_ac_bp = ac.dot(bp);
    if dot_ab_bp >= 0.0 && dot_ac_bp <= dot_ab_bp {
        return b;
    }

    let determinant_c = dot_ab_ap * dot_ac_bp - dot_ab_bp * dot_ac_ap;
    if determinant_c <= 0.0 && dot_ab_ap >= 0.0 && dot_ab_bp <= 0.0 {
        return a + ab * (dot_ab_ap / (dot_ab_ap - dot_ab_bp));
    }

    let cp = point - c;
    let dot_ab_cp = ab.dot(cp);
    let dot_ac_cp = ac.dot(cp);
    if dot_ac_cp >= 0.0 && dot_ab_cp <= dot_ac_cp {
        return c;
    }

    let determinant_b = dot_ab_cp * dot_ac_ap - dot_ab_ap * dot_ac_cp;
    if determinant_b <= 0.0 && dot_ac_ap >= 0.0 && dot_ac_cp <= 0.0 {
        return a + ac * (dot_ac_ap / (dot_ac_ap - dot_ac_cp));
    }

    let determinant_a = dot_ab_bp * dot_ac_cp - dot_ab_cp * dot_ac_bp;
    if determinant_a <= 0.0 && dot_ac_bp - dot_ab_bp >= 0.0 && dot_ab_cp - dot_ac_cp >= 0.0 {
        let edge_parameter =
            (dot_ac_bp - dot_ab_bp) / ((dot_ac_bp - dot_ab_bp) + (dot_ab_cp - dot_ac_cp));
        return b + (c - b) * edge_parameter;
    }

    let denominator = 1.0 / (determinant_a + determinant_b + determinant_c);
    a + ab * (determinant_b * denominator) + ac * (determinant_c * denominator)
}
