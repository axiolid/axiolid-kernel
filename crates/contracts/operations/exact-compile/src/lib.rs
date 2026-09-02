#![forbid(unsafe_code)]
//! Explicit graph-to-exact-B-rep compilation contract.
//!
//! The sibling `axiolid-mesh-compile-contract` lowers a graph to triangles and
//! says so. This contract is its exact counterpart: an implementation either
//! preserves analytic supports and trims, or refuses. There is deliberately no
//! variant that returns a mesh, so a caller asking for exactness cannot be
//! silently handed an approximation.

mod contract;

pub use contract::ExactCompiler;

/// Capability advertised by a graph-to-exact-B-rep implementation.
pub const CAPABILITY_ID: axiolid_contracts::CapabilityId =
    axiolid_contracts::capability_ids::GRAPH_TO_EXACT_BREP;
