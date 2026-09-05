//! An in-plane coordinate frame: the boundary between 3D model space and a
//! plane's own 2D parameter space.
//!
//! # Why this is a type rather than three loose vectors
//!
//! [`Frame2`](crate::primitives::Frame2) and [`Frame3`](crate::primitives::Frame3)
//! are inert storage: they hold axes but carry no
//! behaviour, so every consumer re-derives the same two operations and its own
//! validity rule. That produced four separate orthonormality checks in this
//! workspace under three different tolerance policies, one of which compared a
//! DIMENSIONLESS dot product against the LINEAR tolerance and so scaled a pure
//! direction test with the model length unit.
//!
//! A frame that validates on construction and owns both directions of the map
//! makes that class of bug unrepresentable.

use crate::primitives::{Point2, Point3, Vec3};
use crate::scalar::{Scalar, Tolerance};

/// Why a basis could not form an in-plane frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaneFrameError {
    /// An origin or axis component was NaN or infinite.
    NonFiniteInput,
    /// An axis was not unit length within the angular tolerance.
    NotUnitLength,
    /// The two axes were not perpendicular within the angular tolerance.
    NotPerpendicular,
    /// The axes were parallel, so they span a line rather than a plane.
    Degenerate,
}

impl core::fmt::Display for PlaneFrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::NonFiniteInput => "plane frame origin and axes must be finite",
            Self::NotUnitLength => "plane frame axes must be unit length",
            Self::NotPerpendicular => "plane frame axes must be perpendicular",
            Self::Degenerate => "plane frame axes must span a plane, not a line",
        })
    }
}

impl std::error::Error for PlaneFrameError {}

/// An origin and a right-handed orthonormal pair of in-plane axes.
///
/// # Invariant
///
/// A `PlaneFrame` that exists is orthonormal within the tolerance it was
/// built with. [`project`](Self::project) and [`lift`](Self::lift) are exact
/// inverses on that basis, so the type cannot be used to produce the silently
/// wrong coordinates a skewed basis yields.
///
/// The fields are private precisely because the invariant is the point: a
/// public `x` would let a caller reassign one axis and keep the type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneFrame {
    origin: Point3,
    x: Vec3,
    y: Vec3,
}

impl PlaneFrame {
    /// The z = 0 ground plane with the world x and y axes.
    #[must_use]
    pub const fn ground() -> Self {
        Self {
            origin: Point3::new(0.0, 0.0, 0.0),
            x: Vec3::new(1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
        }
    }

    /// Validate a basis and build a frame from it.
    ///
    /// Unit length and perpendicularity are both DIMENSIONLESS comparisons, so
    /// both use the ANGULAR tolerance. Using the linear tolerance here would
    /// make a pure direction test scale with the model length unit: the same
    /// skewed basis would be refused in metres and accepted in millimetres.
    ///
    /// # Errors
    ///
    /// Returns [`PlaneFrameError`] naming which property failed, so a caller
    /// can report the cause rather than a bare rejection.
    pub fn new(
        origin: Point3,
        x: Vec3,
        y: Vec3,
        tolerance: Tolerance,
    ) -> Result<Self, PlaneFrameError> {
        if !origin.is_finite() || !x.is_finite() || !y.is_finite() {
            return Err(PlaneFrameError::NonFiniteInput);
        }
        // Floor the tolerance at a few ulps: a caller passing Tolerance::ZERO
        // means "exact", but demanding a bit-exact unit length would reject
        // bases that are correct to the limit of f64.
        let unit = tolerance.angular().max(Scalar::EPSILON * 8.0);
        if (x.length_squared() - 1.0).abs() > unit || (y.length_squared() - 1.0).abs() > unit {
            return Err(PlaneFrameError::NotUnitLength);
        }
        if x.cross(y).length_squared() <= unit {
            return Err(PlaneFrameError::Degenerate);
        }
        if x.dot(y).abs() > unit {
            return Err(PlaneFrameError::NotPerpendicular);
        }
        Ok(Self { origin, x, y })
    }

    /// Map a model-space point to its in-plane coordinates.
    ///
    /// Points off the plane project ONTO it: the out-of-plane component is
    /// discarded, not reported. Use [`signed_distance`](Self::signed_distance)
    /// when the caller needs to know how far off the plane the point was.
    #[must_use]
    pub fn project(&self, point: Point3) -> Point2 {
        let offset = point - self.origin;
        Point2::new(offset.dot(self.x), offset.dot(self.y))
    }

    /// Map in-plane coordinates back to model space.
    #[must_use]
    pub fn lift(&self, point: Point2) -> Point3 {
        self.origin + self.x * point.x + self.y * point.y
    }

    /// Signed distance from the plane, positive along the normal.
    #[must_use]
    pub fn signed_distance(&self, point: Point3) -> Scalar {
        (point - self.origin).dot(self.normal())
    }

    /// The right-handed normal, `x` cross `y`.
    ///
    /// Derived rather than stored: a stored normal is a third value that can
    /// disagree with the axes, which is the inconsistency this type exists to
    /// prevent.
    #[must_use]
    pub fn normal(&self) -> Vec3 {
        self.x.cross(self.y)
    }

    /// Build a frame from a normal, choosing an arbitrary in-plane x axis.
    ///
    /// Use when the caller cares about the PLANE but not about which in-plane
    /// direction is x, such as sectioning. When the x direction is meaningful
    /// (a drawing's horizontal, a wall's length), pass it explicitly via
    /// [`new`](Self::new) instead of accepting whatever this picks.
    ///
    /// # Errors
    ///
    /// Returns [`PlaneFrameError`] when the normal is non-finite or too short
    /// to normalize.
    pub fn from_normal(
        origin: Point3,
        normal: Vec3,
        tolerance: Tolerance,
    ) -> Result<Self, PlaneFrameError> {
        if !origin.is_finite() || !normal.is_finite() {
            return Err(PlaneFrameError::NonFiniteInput);
        }
        let unit = tolerance.angular().max(Scalar::EPSILON * 8.0);
        if normal.length_squared() <= unit {
            return Err(PlaneFrameError::Degenerate);
        }
        let z = normal.normalize();
        // Seed against the world axis the normal is LEAST aligned with, so the
        // cross product never approaches zero and the resulting axis is stable.
        let seed = if z.x.abs() <= z.y.abs() && z.x.abs() <= z.z.abs() {
            Vec3::new(1.0, 0.0, 0.0)
        } else if z.y.abs() <= z.z.abs() {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let x = z.cross(seed).normalize();
        Ok(Self {
            origin,
            x,
            y: z.cross(x),
        })
    }

    /// The frame origin.
    #[must_use]
    pub const fn origin(&self) -> Point3 {
        self.origin
    }

    /// The in-plane axis mapped to the planar x coordinate.
    #[must_use]
    pub const fn x_axis(&self) -> Vec3 {
        self.x
    }

    /// The in-plane axis mapped to the planar y coordinate.
    #[must_use]
    pub const fn y_axis(&self) -> Vec3 {
        self.y
    }
}
