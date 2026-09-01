//! Neutral clearance queries along the field's local `z` axis.
//!
//! Every function reports a distance or a blocking fact. None of them decides
//! whether a distance is adequate: "0.9 m of free span" is geometry, "too low"
//! is policy and belongs to the caller.

use axiolid_core::{Interval, Scalar};

use crate::{FieldConfig, LayeredField, LayeredFieldError};

/// Free span found above a reference layer coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearanceReport {
    /// Distance from the reference coordinate to the next blocking layer.
    pub distance: Scalar,
    /// Layer coordinate of the blocking crossing, if the span is bounded.
    pub blocked_at: Option<Scalar>,
    /// Whether the span ends at the field bound rather than at geometry.
    pub bounded_by_field: bool,
}

/// Distance from `w` upward (along local `+z`) to the next surface crossing.
///
/// Crossings within `tolerance.linear()` of `w` are treated as the reference
/// surface itself, not as a blocker, so standing on a slab does not report zero
/// clearance against that same slab.
pub fn clearance_above(
    field: &LayeredField,
    config: &FieldConfig,
    x: usize,
    y: usize,
    w: Scalar,
) -> Result<ClearanceReport, LayeredFieldError> {
    directional_clearance(field, config, x, y, w, true)
}

/// Distance from `w` downward (against local `+z`) to the next surface crossing.
pub fn clearance_below(
    field: &LayeredField,
    config: &FieldConfig,
    x: usize,
    y: usize,
    w: Scalar,
) -> Result<ClearanceReport, LayeredFieldError> {
    directional_clearance(field, config, x, y, w, false)
}

/// Largest unoccupied span in a cell within an explicit search window.
pub fn largest_free_span(
    field: &LayeredField,
    x: usize,
    y: usize,
    search: Interval,
) -> Result<Option<Interval>, LayeredFieldError> {
    let cell = field
        .cell(x, y)
        .ok_or(LayeredFieldError::NodeOutsideField)?;
    Ok(cell.largest_free_span(search))
}

fn directional_clearance(
    field: &LayeredField,
    config: &FieldConfig,
    x: usize,
    y: usize,
    w: Scalar,
    upward: bool,
) -> Result<ClearanceReport, LayeredFieldError> {
    if !w.is_finite() {
        return Err(LayeredFieldError::InvalidInterval);
    }
    let cell = field
        .cell(x, y)
        .ok_or(LayeredFieldError::NodeOutsideField)?;
    let linear = config.tolerance().linear();
    let span = config.bounds().normal_span();
    let bound = if upward { span.end } else { span.start };

    let blocker = cell
        .surfaces()
        .iter()
        .map(|hit| hit.w())
        .filter(|value| {
            if upward {
                *value > w + linear
            } else {
                *value < w - linear
            }
        })
        .fold(None::<Scalar>, |best, value| match best {
            Some(current) if (current - w).abs() <= (value - w).abs() => Some(current),
            _ => Some(value),
        });

    Ok(match blocker {
        Some(value) => ClearanceReport {
            distance: (value - w).abs(),
            blocked_at: Some(value),
            bounded_by_field: false,
        },
        None => ClearanceReport {
            distance: (bound - w).abs(),
            blocked_at: None,
            bounded_by_field: true,
        },
    })
}
