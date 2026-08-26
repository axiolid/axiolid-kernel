#![forbid(unsafe_code)]

//! Spatial acceleration contracts.
//!
//! BVH, octree, GPU broad phase, or a migrated Solibri index can implement the
//! same callback API. Narrow-phase geometry remains outside the index.

pub mod bvh;
pub mod index;

pub use bvh::{Bvh, CandidatePair, NearestCandidate, PairCandidates, SpatialQueryStats};
pub use index::{RayHit, SpatialIndex, SpatialItem};
