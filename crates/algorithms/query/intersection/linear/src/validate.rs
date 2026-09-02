//! Shared input admissibility checks.
//!
//! Validation happens once, on the way in, and names the operand at fault.
//! A predicate cascade cannot rescue a NaN, so refusing early is the only
//! honest behaviour.

use axiolid_core::{Point2, Vec2};

use crate::error::{InputSide, LinearIntersectionError};

pub(crate) fn finite_point(point: Point2, side: InputSide) -> Result<(), LinearIntersectionError> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(LinearIntersectionError::NonFiniteInput { side })
    }
}

pub(crate) fn finite_vector(vector: Vec2, side: InputSide) -> Result<(), LinearIntersectionError> {
    if vector.x.is_finite() && vector.y.is_finite() {
        Ok(())
    } else {
        Err(LinearIntersectionError::NonFiniteInput { side })
    }
}

/// A zero direction does not define a line, so it is refused rather than
/// silently treated as a degenerate parallel case.
pub(crate) fn nonzero_direction(
    direction: Vec2,
    side: InputSide,
) -> Result<(), LinearIntersectionError> {
    if direction.x == 0.0 && direction.y == 0.0 {
        Err(LinearIntersectionError::DegenerateDirection { side })
    } else {
        Ok(())
    }
}
