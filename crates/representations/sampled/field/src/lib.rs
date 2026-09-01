#![forbid(unsafe_code)]
//! Frame-neutral, deterministic layered spatial-field values.
//!
//! A field stores row-major `(x, y)` cells in an explicit frame. Surface
//! crossings and positive-length occupancy spans remain distinct channels so a
//! zero-thickness facet cannot masquerade as filled space. This package owns
//! values, validation, and caller-supplied configuration only; sampling,
//! morphology, clearance, and navigation live in `axiolid-field-ops`.

mod cell;
mod config;
mod error;
mod field;

pub use cell::{LayeredCell, SurfaceFacing, SurfaceHit};
pub use config::{FieldBounds, FieldConfig, FieldResourceBudget};
pub use error::LayeredFieldError;
pub use field::{FieldEvidence, LayeredField};
