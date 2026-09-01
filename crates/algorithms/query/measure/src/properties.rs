//! Measurement result values.

use axiolid_core::{Point3, Scalar, Vec3};

/// Surface/solid mass properties in local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassProperties {
    /// Surface area.
    pub area: Scalar,
    /// Signed volume; orientation determines sign.
    pub signed_volume: Scalar,
    /// Volume centroid.
    pub centroid: Point3,
    /// Diagonal of second moments about the origin.
    pub second_moment_diagonal: Vec3,
}
