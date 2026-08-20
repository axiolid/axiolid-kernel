//! Coordinate, direction, transform, and analytic support types.

use crate::Scalar;

/// Double-precision two-dimensional vector.
pub type Vec2 = glam::DVec2;
/// Double-precision three-dimensional vector.
pub type Vec3 = glam::DVec3;
/// A semantic alias used when a value is a two-dimensional position.
pub type Point2 = Vec2;
/// A semantic alias used when a value is a three-dimensional position.
pub type Point3 = Vec3;
/// Double-precision 3x3 matrix.
pub type Mat3 = glam::DMat3;
/// Double-precision 2D affine transform.
pub type Transform2 = glam::DAffine2;
/// Double-precision affine transform.
pub type Transform3 = glam::DAffine3;
/// Backward-compatible name for [`Transform3`].
pub type Mat4 = Transform3;

/// Right-handed 2D local frame. Algorithms validate orthonormality explicitly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame2 {
    /// Local origin.
    pub origin: Point2,
    /// Local x axis.
    pub x: Vec2,
    /// Local y axis.
    pub y: Vec2,
}

/// Right-handed 3D local frame. Dirty imported frames remain representable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    /// Local origin.
    pub origin: Point3,
    /// Local x axis.
    pub x: Vec3,
    /// Local y axis.
    pub y: Vec3,
    /// Local z axis.
    pub z: Vec3,
}

/// A finite parameter interval. The endpoint order carries orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// Start parameter.
    pub start: Scalar,
    /// End parameter.
    pub end: Scalar,
}

impl Interval {
    /// Unit parameter interval.
    pub const UNIT: Self = Self {
        start: 0.0,
        end: 1.0,
    };

    /// Construct an oriented interval without sorting its endpoints.
    pub const fn new(start: Scalar, end: Scalar) -> Self {
        Self { start, end }
    }

    /// Absolute parameter span.
    pub fn length(self) -> Scalar {
        (self.end - self.start).abs()
    }
}

/// A plane represented by an origin and unit-normal candidate.
///
/// Adapters may construct dirty input. Algorithms validate normalization using
/// the operation's tolerance instead of hiding a global epsilon here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane3 {
    /// Point on the plane.
    pub origin: Point3,
    /// Expected outward normal.
    pub normal: Vec3,
}

/// A parametric three-dimensional ray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3 {
    /// Ray start.
    pub origin: Point3,
    /// Ray direction. It need not be normalized at the storage boundary.
    pub direction: Vec3,
}
