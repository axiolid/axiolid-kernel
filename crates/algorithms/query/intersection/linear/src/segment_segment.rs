//! Bounded segment/segment classification in the plane.
//!
//! A boolean "do they cross" answer is not enough for topology: touching at an
//! endpoint, crossing transversally, and overlapping along a shared span are
//! different facts with different consequences for imprinting and rule checks.

use axiolid_core::{Interval, Point2, Scalar, Tolerance};
use axiolid_guarantees::Sign;
use axiolid_linear::Segment2;
use axiolid_predicates::orient2d;

use crate::error::{InputSide, LinearIntersectionError};
use crate::validate::finite_point;

/// How two bounded planar segments relate.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentSegmentIntersection2 {
    /// The segments share no point.
    Disjoint,
    /// The segments meet at exactly one point.
    ///
    /// Parameters are normalised to `[0, 1]` along each segment, so an
    /// endpoint contact is visible as a parameter of exactly `0.0` or `1.0`.
    Point {
        /// The shared point.
        point: Point2,
        /// Normalised parameter on the left segment.
        left_parameter: Scalar,
        /// Normalised parameter on the right segment.
        right_parameter: Scalar,
    },
    /// The segments are collinear and share a positive-length span.
    Overlap {
        /// Shared span in the left segment's normalised parameter.
        left_interval: Interval,
        /// Shared span in the right segment's normalised parameter.
        right_interval: Interval,
    },
}

/// Classify two bounded planar segments.
pub fn segment_segment2(
    left: Segment2,
    right: Segment2,
    tolerance: Tolerance,
) -> Result<SegmentSegmentIntersection2, LinearIntersectionError> {
    validate(left, InputSide::Left)?;
    validate(right, InputSide::Right)?;

    let (a, b) = (left.start, left.end);
    let (c, d) = (right.start, right.end);

    // Certified side classifications drive every branch below.
    let d1 = sign(orient2d(a, b, c).sign())?;
    let d2 = sign(orient2d(a, b, d).sign())?;
    let d3 = sign(orient2d(c, d, a).sign())?;
    let d4 = sign(orient2d(c, d, b).sign())?;

    let collinear = d1 == Sign::Zero && d2 == Sign::Zero && d3 == Sign::Zero && d4 == Sign::Zero;
    if collinear {
        return collinear_relation(left, right);
    }

    // Straddle test: each segment must separate the other's endpoints, with
    // equality allowed so endpoint contact is reported rather than dropped.
    let left_straddles = straddles(d1, d2);
    let right_straddles = straddles(d3, d4);
    if !(left_straddles && right_straddles) {
        return Ok(SegmentSegmentIntersection2::Disjoint);
    }

    let left_direction = (b.x - a.x, b.y - a.y);
    let right_direction = (d.x - c.x, d.y - c.y);
    let determinant = left_direction.0 * right_direction.1 - left_direction.1 * right_direction.0;
    if !determinant.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }
    if determinant == 0.0 {
        // The predicates proved a crossing, so a zero float determinant means
        // the parameters are not computable in f64; refuse instead of guessing.
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    let delta = (c.x - a.x, c.y - a.y);
    let left_parameter = (delta.0 * right_direction.1 - delta.1 * right_direction.0) / determinant;
    let right_parameter = (delta.0 * left_direction.1 - delta.1 * left_direction.0) / determinant;
    if !left_parameter.is_finite() || !right_parameter.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    // Snap to the certified endpoint facts rather than trusting the division:
    // a zero orientation is proof the endpoint lies on the other segment.
    let left_parameter = snap(left_parameter, d3 == Sign::Zero, d4 == Sign::Zero);
    let right_parameter = snap(right_parameter, d1 == Sign::Zero, d2 == Sign::Zero);

    let point = Point2 {
        x: a.x + left_parameter * left_direction.0,
        y: a.y + left_parameter * left_direction.1,
    };
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    let residual_x = point.x - (c.x + right_parameter * right_direction.0);
    let residual_y = point.y - (c.y + right_parameter * right_direction.1);
    let residual = residual_x.hypot(residual_y);
    let scale = point.x.abs().max(point.y.abs()).max(1.0);
    if !residual.is_finite()
        || (residual > tolerance.linear() * scale && residual > f64::EPSILON * scale * 16.0)
    {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }

    Ok(SegmentSegmentIntersection2::Point {
        point,
        left_parameter,
        right_parameter,
    })
}

/// A certified zero endpoint orientation is stronger evidence than a divided
/// parameter, so it pins the parameter exactly to the endpoint.
fn snap(parameter: Scalar, at_start: bool, at_end: bool) -> Scalar {
    match (at_start, at_end) {
        (true, false) => 0.0,
        (false, true) => 1.0,
        _ => parameter.clamp(0.0, 1.0),
    }
}

fn straddles(first: Sign, second: Sign) -> bool {
    matches!(
        (first, second),
        (Sign::Zero, _)
            | (_, Sign::Zero)
            | (Sign::Positive, Sign::Negative)
            | (Sign::Negative, Sign::Positive)
    )
}

/// Collinear segments overlap on a shared parameter span, touch at one point,
/// or are disjoint. The span is computed by projecting onto the dominant axis,
/// which avoids dividing by a near-zero component.
fn collinear_relation(
    left: Segment2,
    right: Segment2,
) -> Result<SegmentSegmentIntersection2, LinearIntersectionError> {
    let direction = (left.end.x - left.start.x, left.end.y - left.start.y);
    let length_squared = direction.0 * direction.0 + direction.1 * direction.1;
    if !length_squared.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }
    if length_squared == 0.0 {
        return Err(LinearIntersectionError::DegenerateDirection {
            side: InputSide::Left,
        });
    }

    let project = |point: Point2| {
        ((point.x - left.start.x) * direction.0 + (point.y - left.start.y) * direction.1)
            / length_squared
    };
    let right_start = project(right.start);
    let right_end = project(right.end);
    if !right_start.is_finite() || !right_end.is_finite() {
        return Err(LinearIntersectionError::ArithmeticOverflow);
    }
    let (low, high) = if right_start <= right_end {
        (right_start, right_end)
    } else {
        (right_end, right_start)
    };

    let start = low.max(0.0);
    let end = high.min(1.0);
    if start > end {
        return Ok(SegmentSegmentIntersection2::Disjoint);
    }
    if start == end {
        let point = Point2 {
            x: left.start.x + start * direction.0,
            y: left.start.y + start * direction.1,
        };
        let span = right_end - right_start;
        let right_parameter = if span == 0.0 {
            0.0
        } else {
            ((start - right_start) / span).clamp(0.0, 1.0)
        };
        return Ok(SegmentSegmentIntersection2::Point {
            point,
            left_parameter: start,
            right_parameter,
        });
    }

    let span = right_end - right_start;
    if span == 0.0 {
        return Err(LinearIntersectionError::DegenerateDirection {
            side: InputSide::Right,
        });
    }
    let right_low = ((start - right_start) / span).clamp(0.0, 1.0);
    let right_high = ((end - right_start) / span).clamp(0.0, 1.0);
    let (right_low, right_high) = if right_low <= right_high {
        (right_low, right_high)
    } else {
        (right_high, right_low)
    };

    let left_interval = Interval::new(start, end);
    let right_interval = Interval::new(right_low, right_high);
    Ok(SegmentSegmentIntersection2::Overlap {
        left_interval,
        right_interval,
    })
}

/// A certified predicate must produce a sign; an uncertain answer is a refusal.
fn sign(sign: Option<Sign>) -> Result<Sign, LinearIntersectionError> {
    sign.ok_or(LinearIntersectionError::ArithmeticOverflow)
}

fn validate(segment: Segment2, side: InputSide) -> Result<(), LinearIntersectionError> {
    finite_point(segment.start, side)?;
    finite_point(segment.end, side)?;
    if segment.start == segment.end {
        return Err(LinearIntersectionError::DegenerateDirection { side });
    }
    Ok(())
}
