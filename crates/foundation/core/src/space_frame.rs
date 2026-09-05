//! A full 3D orthonormal frame: the boundary between model space and a
//! local coordinate system with three axes.
//!
//! # Why this exists alongside PlaneFrame
//!
//! PlaneFrame answers "where is this point on that plane". A SpaceFrame
//! answers "what are this point coordinates in that local system", keeping
//! the third axis. Surface evaluation, sampled fields, and sectioning all
//! need the second question.
//!
//! Three call sites previously each rolled their own validity rule under a
//! DIFFERENT tolerance policy, and only two of the three checked handedness.
//! A left-handed basis passes every unit-length and perpendicularity test
//! while mirroring the geometry, so omitting that check silently reflects
//! whatever the frame maps.

use crate::plane_frame::FrameError;
use crate::primitives::{Point3, Vec3};
use crate::scalar::{Scalar, Tolerance};

/// An origin and a right-handed orthonormal triad of axes.
///
/// # Invariant
///
/// A `SpaceFrame` that exists is valid: finite, unit-length on every axis,
/// mutually perpendicular, and right-handed, all judged when it was built.
/// Fields are private so a struct literal cannot bypass that.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceFrame {
    origin: Point3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl SpaceFrame {
    /// The world axes at the origin.
    #[must_use]
    pub const fn world() -> Self {
        Self {
            origin: Point3::new(0.0, 0.0, 0.0),
            x: Vec3::new(1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
        }
    }

    /// Validate a basis and build a frame from it.
    ///
    /// Unit length, perpendicularity, and handedness are all DIMENSIONLESS
    /// properties, so all three are judged against the ANGULAR tolerance.
    /// Using the linear tolerance would tie a pure direction test to the
    /// model length unit.
    ///
    /// Handedness is checked explicitly because it is invisible to the other
    /// two tests: a mirrored basis is perfectly orthonormal and still
    /// reflects every point it maps.
    ///
    /// # Errors
    ///
    /// Returns the property that failed, so a caller can name the cause.
    pub fn new(
        origin: Point3,
        x: Vec3,
        y: Vec3,
        z: Vec3,
        tolerance: Tolerance,
    ) -> Result<Self, FrameError> {
        if !origin.is_finite() || !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(FrameError::NonFiniteInput);
        }
        // Floored at a few ulps so Tolerance::ZERO still admits bases that
        // are correct to the limit of f64.
        let unit = tolerance.angular().max(Scalar::EPSILON * 8.0);
        let lengths = [x.length_squared(), y.length_squared(), z.length_squared()];
        if lengths.iter().any(|l| (l - 1.0).abs() > unit) {
            return Err(FrameError::NotUnitLength);
        }
        if x.dot(y).abs() > unit || y.dot(z).abs() > unit || z.dot(x).abs() > unit {
            return Err(FrameError::NotPerpendicular);
        }
        // On an orthonormal triad this triple product is exactly +1 for a
        // right-handed basis and -1 for a mirrored one, so comparing against
        // +1 separates them by a margin of 2 rather than by a tolerance.
        if (x.cross(y).dot(z) - 1.0).abs() > unit {
            return Err(FrameError::NotRightHanded);
        }
        Ok(Self { origin, x, y, z })
    }

    /// Map a model-space point into local coordinates.
    ///
    /// Exact inverse of [`to_world`](Self::to_world): the frame is orthonormal,
    /// so the inverse of the rotation is its transpose, which is a projection
    /// onto the axes.
    #[must_use]
    pub fn to_local(&self, point: Point3) -> Vec3 {
        let offset = point - self.origin;
        Vec3::new(offset.dot(self.x), offset.dot(self.y), offset.dot(self.z))
    }

    /// Map local coordinates back into model space.
    #[must_use]
    pub fn to_world(&self, local: Vec3) -> Point3 {
        self.origin + self.x * local.x + self.y * local.y + self.z * local.z
    }

    /// The frame origin.
    #[must_use]
    pub const fn origin(&self) -> Point3 {
        self.origin
    }

    /// The local x axis.
    #[must_use]
    pub const fn x(&self) -> Vec3 {
        self.x
    }

    /// The local y axis.
    #[must_use]
    pub const fn y(&self) -> Vec3 {
        self.y
    }

    /// The local z axis.
    #[must_use]
    pub const fn z(&self) -> Vec3 {
        self.z
    }
}
