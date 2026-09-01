//! Open diagnosis and repair extension points.

use axiolid_core::Tolerance;

use crate::{Diagnosis, RepairPlan, RepairReport};

/// Diagnose without mutating input.
pub trait Diagnose<T>: core::fmt::Debug + Send + Sync {
    /// Structured failure to complete diagnosis.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Inspect one value.
    fn diagnose(&self, value: &T, tolerance: Tolerance) -> Result<Diagnosis, Self::Error>;
}

/// Apply one explicit repair plan and return new geometry plus audit report.
pub trait Repair<T>: core::fmt::Debug + Send + Sync {
    /// Structured repair failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Never mutates the caller's source value in place.
    fn repair(
        &self,
        value: &T,
        plan: &RepairPlan,
        tolerance: Tolerance,
    ) -> Result<(T, RepairReport), Self::Error>;
}
