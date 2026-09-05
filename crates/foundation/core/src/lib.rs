#![forbid(unsafe_code)]

//! Shared geometry value types.
//!
//! This is the dependency root: data only, no algorithms, no source-format
//! semantics, no serialization policy, and no hardware backend. Coordinates use
//! `f64`; every tolerance-sensitive operation receives an explicit [`Tolerance`].

pub mod bounds;
pub mod operation;
pub mod plane_frame;
pub mod primitives;
pub mod scalar;

pub use bounds::Aabb;
pub use operation::BooleanOperator;
pub use plane_frame::{PlaneFrame, PlaneFrameError};
pub use primitives::{
    Frame2, Frame3, Interval, Mat3, Mat4, Plane3, Point2, Point3, Ray3, Transform2, Transform3,
    Vec2, Vec3,
};
pub use scalar::{Scalar, Tolerance, ToleranceError};
