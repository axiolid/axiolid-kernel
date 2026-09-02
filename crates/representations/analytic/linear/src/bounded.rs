//! Bounded and half-bounded linear geometry.
//!
//! `Ray3` is deliberately **not** redefined here: `axiolid_core::Ray3` already
//! owns that value, and a second incompatible `Ray3` would split the vocabulary
//! for every existing consumer. It is re-exported instead, so this package
//! still presents a complete linear vocabulary without duplicating a type.

use axiolid_core::{Point2, Point3, Vec2};

pub use axiolid_core::Ray3;

/// A parametric two-dimensional ray: `origin + t * direction` for `t >= 0`.
///
/// The 3D counterpart lives in `axiolid-core`; this is the missing planar half
/// of the pair, added because planar linear queries need a stable value type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray2 {
    /// Ray start.
    pub origin: Point2,
    /// Ray direction. It need not be normalized at the storage boundary.
    pub direction: Vec2,
}

/// A bounded linear span between two endpoints.
///
/// A segment is stored by its endpoints rather than an origin/direction pair
/// with an interval, because endpoint identity is what topology and imported
/// data actually carry. The natural parameterisation is `start + t * (end -
/// start)` over `t` in `[0, 1]`; keeping that implicit avoids inventing a
/// parameter range the source never stated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment<P> {
    /// Endpoint at parameter zero.
    pub start: P,
    /// Endpoint at parameter one.
    pub end: P,
}

/// Two-dimensional segment.
pub type Segment2 = Segment<Point2>;
/// Three-dimensional segment.
pub type Segment3 = Segment<Point3>;
