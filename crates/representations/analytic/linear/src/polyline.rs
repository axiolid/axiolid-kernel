//! Piecewise-linear curves.

use axiolid_core::{Point2, Point3};

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
