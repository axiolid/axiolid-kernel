//! Shared validation for compact non-uniform B-spline axes.
//!
//! Imported CAD data stores distinct knots and parallel multiplicities. Validate
//! that compact encoding before expanding it: `zip`-then-check silently drops
//! unmatched entries, and expanding an attacker-controlled multiplicity before
//! checking the expected total is an allocation denial of service.

use axiolid_core::Scalar;
use axiolid_kernel::{GeomError, GeomResult};

/// Validated expanded knot axis used by the scalar de Boor evaluators.
pub(crate) struct SplineAxis {
    pub(crate) knots: Vec<Scalar>,
    pub(crate) count: usize,
    pub(crate) degree: usize,
}

impl SplineAxis {
    pub(crate) fn new(
        knots: &[Scalar],
        multiplicities: &[u32],
        degree: u16,
        count: usize,
        label: &str,
    ) -> GeomResult<Self> {
        let degree = usize::from(degree);
        if degree == 0 {
            return Err(GeomError::InvalidInput(format!(
                "{label} degree must be at least 1"
            )));
        }
        if count <= degree {
            return Err(GeomError::InvalidInput(format!(
                "{label} degree {degree} requires at least {} control points, got {count}",
                degree + 1
            )));
        }
        if knots.len() != multiplicities.len() {
            return Err(GeomError::InvalidInput(format!(
                "{label} has {} distinct knots but {} multiplicities",
                knots.len(),
                multiplicities.len()
            )));
        }
        if knots.len() < 2 {
            return Err(GeomError::InvalidInput(format!(
                "{label} needs at least two distinct knots"
            )));
        }
        if knots.iter().any(|knot| !knot.is_finite()) {
            return Err(GeomError::InvalidInput(format!(
                "{label} knots must be finite"
            )));
        }
        if knots.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(GeomError::InvalidInput(format!(
                "{label} distinct knots must be strictly increasing"
            )));
        }

        let expected = count
            .checked_add(degree)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| GeomError::InvalidInput(format!("{label} size overflows usize")))?;
        let maximum_multiplicity = degree + 1;
        let mut total = 0usize;
        for &multiplicity in multiplicities {
            let multiplicity = usize::try_from(multiplicity).map_err(|_| {
                GeomError::InvalidInput(format!("{label} multiplicity does not fit usize"))
            })?;
            if multiplicity == 0 || multiplicity > maximum_multiplicity {
                return Err(GeomError::InvalidInput(format!(
                    "{label} multiplicity {multiplicity} is outside 1..={maximum_multiplicity}"
                )));
            }
            total = total.checked_add(multiplicity).ok_or_else(|| {
                GeomError::InvalidInput(format!("{label} multiplicity sum overflows usize"))
            })?;
            if total > expected {
                return Err(GeomError::InvalidInput(format!(
                    "{label} knot vector has more than the expected {expected} entries"
                )));
            }
        }
        if total != expected {
            return Err(GeomError::InvalidInput(format!(
                "{label} knot vector has {total} entries, expected {expected}"
            )));
        }

        // Capacity is now bounded by the control count and degree, not raw
        // imported multiplicities.
        let mut expanded = Vec::with_capacity(expected);
        for (&knot, &multiplicity) in knots.iter().zip(multiplicities) {
            expanded.extend(core::iter::repeat_n(knot, multiplicity as usize));
        }
        let lo = expanded[degree];
        let hi = expanded[count];
        if hi <= lo {
            return Err(GeomError::Degenerate(format!("{label} domain is empty")));
        }

        Ok(Self {
            knots: expanded,
            count,
            degree,
        })
    }

    pub(crate) fn domain(&self) -> (Scalar, Scalar) {
        (self.knots[self.degree], self.knots[self.count])
    }

    pub(crate) fn clamp(&self, parameter: Scalar) -> Scalar {
        let (lo, hi) = self.domain();
        parameter.clamp(lo, hi)
    }
}
