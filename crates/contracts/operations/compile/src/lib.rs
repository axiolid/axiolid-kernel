#![forbid(unsafe_code)]
//! Explicit graph-to-triangle-mesh compilation contract.
//!
//! This operation never claims to preserve an exact B-rep result domain.

mod contract;

pub use contract::MeshCompiler;

pub const CAPABILITY_ID: axiolid_contracts::CapabilityId =
    axiolid_contracts::capability_ids::GRAPH_TO_MESH;
