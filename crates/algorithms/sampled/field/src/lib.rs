#![forbid(unsafe_code)]
//! Format-neutral algorithms over [`axiolid_field`] values.
//!
//! This package reports coverage, spans, components, and route existence under
//! caller-supplied geometric envelopes. It does not attach application verdicts
//! to those facts.

mod clearance;
mod morphology;
mod sample;

#[cfg(feature = "navigation")]
pub mod navigate;

pub use axiolid_field::{
    FieldBounds, FieldConfig, FieldEvidence, FieldResourceBudget, LayeredCell, LayeredField,
    LayeredFieldError, SurfaceFacing, SurfaceHit,
};
pub use clearance::{clearance_above, clearance_below, largest_free_span, ClearanceReport};
pub use morphology::{ComponentLabels, FieldChannel, PlanarMask};
pub use sample::{sample_triangles_cpu, CpuCoverageProvider, Triangle3};
