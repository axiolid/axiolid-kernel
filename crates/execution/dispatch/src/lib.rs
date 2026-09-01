#![forbid(unsafe_code)]
//! Runtime provider registration and dispatch policy.
//!
//! Portable request/result schemas live in operation-contract packages. This
//! crate owns ordering, device matching, fallback, and budget admission.

#[cfg(feature = "mesh-boolean")]
mod boolean;
#[cfg(feature = "mesh-section")]
mod section;

#[cfg(feature = "mesh-boolean")]
pub use boolean::MeshBooleanRegistry;
#[cfg(feature = "mesh-section")]
pub use section::MeshPlaneSectionRegistry;
