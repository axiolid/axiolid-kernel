//! Scalar storage and explicit tolerance policy.

use core::fmt;

/// Coordinate scalar stored by the format-neutral model.
///
/// `f64` preserves millimetre detail at national-grid coordinates. A backend may
/// use narrower arithmetic internally only when its reported precision and the
/// requested tolerance make that safe.
pub type Scalar = f64;

/// Invalid tolerance input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToleranceError {
    /// A component was negative, infinite, or NaN.
    NotFiniteAndNonNegative,
}

impl fmt::Display for ToleranceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFiniteAndNonNegative => {
                f.write_str("tolerance values must be finite and non-negative")
            }
        }
    }
}

impl std::error::Error for ToleranceError {}

/// Linear and angular tolerance carried with an operation.
///
/// There is deliberately no [`Default`] implementation: a useful tolerance is
/// a property of the source unit scale and requested accuracy, not the crate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    linear: Scalar,
    angular: Scalar,
}

impl Tolerance {
    /// Exact-coordinate policy. Use only when the caller intentionally wants no
    /// scale-derived tolerance, such as compatibility validation of an existing
    /// source model.
    pub const ZERO: Self = Self {
        linear: 0.0,
        angular: 0.0,
    };

    /// One micrometre linear and one nanoradian angular tolerance when geometry
    /// has already been normalized to metres.
    pub const METRE: Self = Self {
        linear: 1e-6,
        angular: 1e-9,
    };

    /// One micrometre linear and one nanoradian angular tolerance while values
    /// are still expressed in millimetres.
    pub const MILLIMETRE: Self = Self {
        linear: 1e-3,
        angular: 1e-9,
    };

    /// Construct a validated policy.
    pub fn new(linear: Scalar, angular: Scalar) -> Result<Self, ToleranceError> {
        if !linear.is_finite() || !angular.is_finite() || linear < 0.0 || angular < 0.0 {
            return Err(ToleranceError::NotFiniteAndNonNegative);
        }
        Ok(Self { linear, angular })
    }

    /// Absolute distance tolerance in the model's current length unit.
    #[inline]
    pub const fn linear(self) -> Scalar {
        self.linear
    }

    /// Absolute angular tolerance in radians.
    #[inline]
    pub const fn angular(self) -> Scalar {
        self.angular
    }

    /// Compare two scalar values using the linear component.
    #[inline]
    pub fn eq(self, a: Scalar, b: Scalar) -> bool {
        (a - b).abs() <= self.linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_policy() {
        assert_eq!(
            Tolerance::new(Scalar::NAN, 0.0),
            Err(ToleranceError::NotFiniteAndNonNegative)
        );
        assert_eq!(
            Tolerance::new(-1.0, 0.0),
            Err(ToleranceError::NotFiniteAndNonNegative)
        );
    }

    #[test]
    fn policy_has_no_context_free_default() {
        assert!(Tolerance::METRE.eq(1.0, 1.0 + 0.5e-6));
        assert!(!Tolerance::METRE.eq(1.0, 1.0 + 2.0e-6));
    }
}
