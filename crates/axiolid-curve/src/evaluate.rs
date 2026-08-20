//! Backend-open curve evaluation capability.

use axiolid_core::{Interval, Scalar, Tolerance};

/// Evaluate and differentiate one curve representation.
///
/// The trait is generic over the curve value so third-party backends can add
/// implementations without a central enum or source-format dependency.
pub trait CurveEvaluator<C>: core::fmt::Debug + Send + Sync {
    /// Point returned by evaluation.
    type Point;
    /// First derivative returned by evaluation.
    type Derivative;
    /// Evaluator-specific structured error.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Supported parameter interval.
    fn domain(&self, curve: &C) -> Interval;

    /// Position at parameter `t`.
    fn evaluate(
        &self,
        curve: &C,
        t: Scalar,
        tolerance: Tolerance,
    ) -> Result<Self::Point, Self::Error>;

    /// First derivative at parameter `t`.
    fn derivative(
        &self,
        curve: &C,
        t: Scalar,
        tolerance: Tolerance,
    ) -> Result<Self::Derivative, Self::Error>;
}
