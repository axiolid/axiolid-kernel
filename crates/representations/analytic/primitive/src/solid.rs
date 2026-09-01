//! Bounded analytic solid primitives.

use axiolid_core::Scalar;

/// Exact primitive solid; tessellation is a separate operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    /// Axis-aligned box in local coordinates.
    Block {
        /// Extent along x.
        x: Scalar,
        /// Extent along y.
        y: Scalar,
        /// Extent along z.
        z: Scalar,
    },
    /// Sphere centered at the local origin.
    Sphere {
        /// Radius.
        radius: Scalar,
    },
    /// Cylinder along local +z.
    Cylinder {
        /// Radius.
        radius: Scalar,
        /// Height.
        height: Scalar,
    },
    /// Cone along local +z.
    Cone {
        /// Base radius.
        radius: Scalar,
        /// Height.
        height: Scalar,
    },
    /// Rectangular pyramid along local +z.
    Pyramid {
        /// Base extent along x.
        x: Scalar,
        /// Base extent along y.
        y: Scalar,
        /// Height.
        height: Scalar,
    },
}
