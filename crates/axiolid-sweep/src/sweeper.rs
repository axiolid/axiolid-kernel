//! Sweep capability over neutral graph instructions.

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, SolidOperation};

/// Backend-open sweep construction.
pub trait Sweeper: core::fmt::Debug + Send + Sync {
    /// Structured failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Construct one extrusion, revolution, directrix sweep, or sectioned solid.
    fn sweep(
        &self,
        graph: &GeometryGraph,
        operation: &SolidOperation,
        tolerance: Tolerance,
    ) -> Result<TriMesh, Self::Error>;
}
