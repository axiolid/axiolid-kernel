//! Line/line classification in the plane.
//!
//! The parallel-versus-coincident distinction is decided by certified
//! predicates, never by comparing a determinant against a global epsilon. Two
//! lines that are parallel and two that are the same line are different
//! topological facts, and a rounding artefact must not be able to swap them.

use axiolid_core::{Point2, Scalar, Tolerance, Vec2};
use axiolid_guarantees::Sign;
use axiolid_linear::Line2;
use axiolid_predicates::orient2d;

use crate::error::{InputSide, LinearIntersectionError};
use crate::validate::{finite_point, finite_vector, nonzero_direction};

/// How two unbounded planar lines relate.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineLineIntersection2 {
    /// The lines cross at exactly one point.
    Point {
        /// The intersection point.
        point: Point2,
        /// Parameter on the left line.
        left_parameter: Scalar,
        /// Parameter on the right line.
        right_parameter: Scalar,
    },
    /// The lines have the same direction and never meet.
    Parallel,
    /// The lines are the same line.
    Coincident,
}

/// Classify two unbounded planar lines.
///
/// `tolerance` governs the residual acceptance of the computed point, not the
/// topological parallel/coincident decision: that decision uses certified
/// orientation predicates so it cannot be flipped by a scaled epsilon.
pub fn line_line2(
    left: Line2,
    right: Line2,
    tolerance: Tolerance,
) -> Result<LineLineIntersection2, LinearIntersectionError> {
    validate(left, InputSide::Left)?;
    validate(right, InputSide::Right)?;

    // Parallelism is a property of the DIRECTIONS alone. Sampling a point from
    // each line and comparing sides is wrong for unbounded lines: two crossing
    // lines can easily place both sample points on the same side.
    let direction_origin = left.origin;
    let left_tip = translate(direction_origin, left.direction)?;
    let right_tip = translate(direction_origin, right.direction)?;
    let directions_parallel =
        sign(orient2d(direction_origin, left_tip, right_tip).sign())? == Sign::Zero;

    if directions_parallel {
        // Same direction: either the same line, or a disjoint translate of it.
        let left_second = translate(left.origin, left.direction)?;
        let right_on_left = sign(orient2d(left.origin, left_second, right.origin).sign())?;
        return Ok(if right_on_left == Sign::Zero {
            LineLineIntersection2::Coincident
        } else {
            LineLineIntersection2::Parallel
        });
    }

    let determinant = left.direction.x * right.direction.y - left.direction.y * right.direction.x;
    if !determinant.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }
    if determinant == 0.0 {
        // The predicate proved the directions are not collinear, so a zero
        // float determinant means the parameters are not computable in f64.
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    let delta_x = right.origin.x - left.origin.x;
    let delta_y = right.origin.y - left.origin.y;
    let left_parameter = (delta_x * right.direction.y - delta_y * right.direction.x) / determinant;
    let right_parameter = (delta_x * left.direction.y - delta_y * left.direction.x) / determinant;
    if !left_parameter.is_finite() || !right_parameter.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    let point = Point2 {
        x: left.origin.x + left_parameter * left.direction.x,
        y: left.origin.y + left_parameter * left.direction.y,
    };
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    // Residual check against the caller's tolerance: the classification is
    // certified, but the returned coordinate is still a rounded value and the
    // caller asked for a specific accuracy.
    let residual_x = point.x - (right.origin.x + right_parameter * right.direction.x);
    let residual_y = point.y - (right.origin.y + right_parameter * right.direction.y);
    let residual = residual_x.hypot(residual_y);
    if !residual.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }
    let scale = point.x.abs().max(point.y.abs()).max(1.0);
    if residual > tolerance.linear() * scale && residual > f64::EPSILON * scale * 16.0 {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    Ok(LineLineIntersection2::Point {
        point,
        left_parameter,
        right_parameter,
    })
}

fn validate(line: Line2, side: InputSide) -> Result<(), LinearIntersectionError> {
    finite_point(line.origin, side)?;
    finite_vector(line.direction, side)?;
    nonzero_direction(line.direction, side)
}

/// Offset a point by a vector, refusing a non-finite result rather than
/// letting an infinity reach a predicate.
fn translate(point: Point2, offset: Vec2) -> Result<Point2, LinearIntersectionError> {
    let moved = Point2 {
        x: point.x + offset.x,
        y: point.y + offset.y,
    };
    if moved.x.is_finite() && moved.y.is_finite() {
        Ok(moved)
    } else {
        Err(LinearIntersectionError::ArithmeticOverflow)
    }
}

/// A certified predicate must produce a sign; uncertainty is a refusal.
fn sign(sign: Option<Sign>) -> Result<Sign, LinearIntersectionError> {
    sign.ok_or(LinearIntersectionError::ArithmeticOverflow)
}
