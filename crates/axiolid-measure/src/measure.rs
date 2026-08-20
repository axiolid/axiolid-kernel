//! Open measurement capability.

use axiolid_core::Tolerance;

use crate::MassProperties;

/// Compute mass properties for one representation type.
pub trait Measure<T>: core::fmt::Debug + Send + Sync {
    /// Structured failure (open shell, self intersection, unsupported shape).
    type Error: std::error::Error + Send + Sync + 'static;

    /// Measure under an explicit geometric tolerance.
    fn measure(&self, value: &T, tolerance: Tolerance) -> Result<MassProperties, Self::Error>;
}
