//! Profile validation and algorithm extension points.

use axiolid_core::Tolerance;

/// Validate a profile without mutating it.
pub trait ValidateProfile<P> {
    /// Structured validation report.
    type Report: core::fmt::Debug;

    /// Diagnose closure, dimensions, self-intersection, and nesting.
    fn validate(&self, profile: &P, tolerance: Tolerance) -> Self::Report;
}
