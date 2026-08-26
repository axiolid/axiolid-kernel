#![forbid(unsafe_code)]

//! Feature-gated facade for Axiolid geometry.
//!
//! The default build is intentionally small: core values, meshes, and the
//! portable CPU backend shell. Exact curves/surfaces/topology, algorithms,
//! parallel scheduling, and GPU adapters are opt-in. Leaf crates remain public
//! for consumers that want an even narrower dependency graph.

/// Always-available scalar, transform, and bounds vocabulary.
pub mod core {
    pub use axiolid_core::*;
}

pub use axiolid_core::{Aabb, Point2, Point3, Scalar, Tolerance, Transform3, Vec2, Vec3};

#[cfg(feature = "mesh")]
pub mod mesh {
    pub use axiolid_mesh::*;
}

#[cfg(feature = "profiles")]
pub mod profile {
    pub use axiolid_profile::*;
}

#[cfg(feature = "curves")]
pub mod curve {
    pub use axiolid_curve::*;
}

#[cfg(feature = "surfaces")]
pub mod surface {
    pub use axiolid_surface::*;
}

#[cfg(feature = "topology")]
pub mod topology {
    pub use axiolid_topology::*;
}

#[cfg(feature = "model")]
pub mod model {
    pub use axiolid_model::*;
}

#[cfg(feature = "primitives")]
pub mod primitive {
    pub use axiolid_primitive::*;
}

#[cfg(feature = "sweeps")]
pub mod sweep {
    pub use axiolid_sweep::*;
}

#[cfg(feature = "tessellation")]
pub mod tessellation {
    pub use axiolid_tessellate::*;
}

#[cfg(feature = "spatial")]
pub mod spatial {
    pub use axiolid_spatial::*;
}

#[cfg(feature = "measure")]
pub mod measure {
    pub use axiolid_measure::*;
}

#[cfg(feature = "overlay")]
pub mod overlay {
    pub use axiolid_overlay::*;
}

/// Frame-neutral sampled layered fields.
///
/// Coverage, morphology, and clearance are always available with this feature.
/// Geometry-only traversal is a further opt-in via `field-navigation`.
#[cfg(feature = "field")]
pub mod field {
    pub use axiolid_field::*;
}

#[cfg(feature = "heal")]
pub mod heal {
    pub use axiolid_heal::*;
}

#[cfg(feature = "kernel")]
pub mod kernel {
    pub use axiolid_kernel::*;
}

#[cfg(feature = "cpu")]
pub mod cpu {
    pub use axiolid_backend_cpu::*;
}

#[cfg(feature = "gpu")]
pub mod gpu {
    pub use axiolid_backend_gpu::*;
}
