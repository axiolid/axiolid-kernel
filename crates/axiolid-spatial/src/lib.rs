#![forbid(unsafe_code)]

//! Spatial acceleration contracts.
//!
//! BVH, octree, GPU broad phase, or a migrated Solibri index can implement the
//! same callback API. Narrow-phase geometry remains outside the index.

pub mod index;

pub use index::{RayHit, SpatialIndex, SpatialItem};
