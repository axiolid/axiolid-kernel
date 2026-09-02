#![forbid(unsafe_code)]

//! Exact-to-discrete tessellation contracts.
//!
//! Shared topological edges must be discretized once and reused by adjacent
//! faces; per-face independent tessellation is not watertight.

pub mod options;
mod output;
pub mod tessellator;

pub use options::{InvalidTessellationOptions, TessellationOptions};
pub use output::TessellatedMesh;

pub const CAPABILITY_ID: axiolid_contracts::CapabilityId =
    axiolid_contracts::capability_ids::TESSELLATE;
pub use tessellator::Tessellator;
