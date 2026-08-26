#![forbid(unsafe_code)]
//! Frame-neutral, deterministic layered spatial fields.
//!
//! # Model
//!
//! A field is a row-major grid of `(x, y)` cells expressed in an explicit
//! [`axiolid_core::Frame3`]. Each cell is a column along the frame's local `z`
//! axis holding zero, one, or many layers. There is no notion of "the floor" of
//! a cell, no world axis assumption, and no implicit unit or epsilon: the
//! frame, bounds, cell size, tolerance, and resource budget are all supplied by
//! the caller in [`FieldConfig`].
//!
//! Two layer channels are kept separate because they are different geometry:
//!
//! - [`SurfaceHit`] — a zero-thickness crossing produced by triangle coverage.
//! - [`axiolid_core::Interval`] — a positive-length occupied span, produced only
//!   by an explicit closed-shell construction.
//!
//! Collapsing them would let a single facet masquerade as filled space.
//!
//! # Policy boundary
//!
//! This crate reports geometry: distances, spans, components, and route
//! existence under a caller-supplied envelope. It never encodes accessibility, NOT-A-VERDICT
//! egress, compliance, or product rules. "No route under this envelope" is a NOT-A-VERDICT
//! geometric statement; "not accessible" is a consumer's conclusion. NOT-A-VERDICT
//!
//! # Example
//!
//! ```
//! use axiolid_core::{Frame3, Tolerance, Vec3};
//! use axiolid_field::{
//!     sample_triangles_cpu, FieldBounds, FieldConfig, FieldResourceBudget, Triangle3,
//! };
//!
//! let frame = Frame3 { origin: Vec3::ZERO, x: Vec3::X, y: Vec3::Y, z: Vec3::Z };
//! let bounds = FieldBounds::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 4.0)).unwrap();
//! let config = FieldConfig::new(
//!     frame,
//!     bounds,
//!     1.0,
//!     Tolerance::METRE,
//!     FieldResourceBudget::new(16, 64),
//! )
//! .unwrap();
//!
//! let slab = Triangle3::new(
//!     Vec3::new(-1.0, -1.0, 2.0),
//!     Vec3::new(3.0, -1.0, 2.0),
//!     Vec3::new(-1.0, 3.0, 2.0),
//! );
//! let field = sample_triangles_cpu(&config, &[slab]).unwrap();
//! assert_eq!(field.cell(0, 0).unwrap().surfaces().len(), 1);
//! assert!(field.cell(0, 0).unwrap().occupancy().is_empty());
//! ```

mod cell;
mod clearance;
mod config;
mod error;
mod field;
mod morphology;
mod sample;

#[cfg(feature = "navigation")]
pub mod navigate;

pub use cell::{LayeredCell, SurfaceFacing, SurfaceHit};
pub use clearance::{clearance_above, clearance_below, largest_free_span, ClearanceReport};
pub use config::{FieldBounds, FieldConfig, FieldResourceBudget};
pub use error::LayeredFieldError;
pub use field::{FieldEvidence, LayeredField};
pub use morphology::{ComponentLabels, FieldChannel, PlanarMask};
pub use sample::{sample_triangles_cpu, CpuCoverageProvider, Triangle3};
