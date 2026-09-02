#![forbid(unsafe_code)]

//! Format-neutral linear geometry values.
//!
//! This package exists so that a line-query application can compile lines,
//! rays, segments, and polylines without acquiring the general curve
//! aggregate, surfaces, meshes, topology, or any execution machinery. It holds
//! data only: no evaluation policy, no tolerance decisions, no algorithms.
//!
//! `axiolid-curve` re-exports these types, so `axiolid_curve::Line2` and
//! `axiolid_linear::Line2` name the same value.

mod bounded;
mod line;
mod polyline;

/// Coordinate vocabulary, re-exported from `axiolid-core`.
///
/// These are the same types, not copies: a consumer of the narrow linear
/// closure should not need a second dependency to name a point.
pub use axiolid_core::{Point2, Point3, Scalar, Tolerance, Vec2, Vec3};
pub use bounded::{Ray2, Ray3, Segment, Segment2, Segment3};
pub use line::{Line, Line2, Line3};
pub use polyline::{Polyline, Polyline2, Polyline3};
