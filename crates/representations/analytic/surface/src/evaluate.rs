//! Backend-open surface evaluation capability.

use axiolid_core::{Point3, Scalar, Tolerance, Vec3};

/// Evaluate one surface representation without coupling it to a kernel.
pub trait SurfaceEvaluator<S>: core::fmt::Debug + Send + Sync {
    /// Evaluator-specific structured error.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Position at parameters `(u, v)`.
    fn evaluate(
        &self,
        surface: &S,
        u: Scalar,
        v: Scalar,
        tolerance: Tolerance,
    ) -> Result<Point3, Self::Error>;

    /// Surface normal at parameters `(u, v)`.
    fn normal(
        &self,
        surface: &S,
        u: Scalar,
        v: Scalar,
        tolerance: Tolerance,
    ) -> Result<Vec3, Self::Error>;
}
