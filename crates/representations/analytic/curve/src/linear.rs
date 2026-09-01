//! Linear and polyline curve data.

use axiolid_core::{Point2, Point3, Vec2, Vec3};

/// Infinite parametric line `origin + t * direction`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line<P, V> {
    /// Point at parameter zero.
    pub origin: P,
    /// Parameter direction; import adapters may preserve a non-unit vector.
    pub direction: V,
}

/// Two-dimensional line.
pub type Line2 = Line<Point2, Vec2>;
/// Three-dimensional line.
pub type Line3 = Line<Point3, Vec3>;

/// Piecewise-linear curve preserving source vertex order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Polyline<P> {
    /// Ordered control points.
    pub points: Vec<P>,
    /// Whether the final point connects back to the first.
    pub closed: bool,
}

/// Two-dimensional polyline.
pub type Polyline2 = Polyline<Point2>;
/// Three-dimensional polyline.
pub type Polyline3 = Polyline<Point3>;
