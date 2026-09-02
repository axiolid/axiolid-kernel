//! Reference graph-to-exact-B-rep compiler.
//!
//! This is the exact counterpart to [`crate::ReferenceMeshCompiler`]. It
//! advertises `GRAPH_TO_EXACT_BREP` and currently refuses every node: no
//! construction family can yet populate supports and trims without
//! approximation.
//!
//! Refusing is the point. The alternative -- quietly tessellating and calling
//! the result exact -- is the failure mode ADR 0020 exists to prevent. When a
//! family becomes exactly constructible, it is handled here and the refusal
//! narrows to the families still out of reach.

use axiolid_brep::ExactBRep;
use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, Operation,
};
use axiolid_exact_compile_contract::ExactCompiler;
use axiolid_model::{GeometryGraph, NodeId};

/// Scalar reference implementation of the exact-compilation capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReferenceExactCompiler;

impl ReferenceExactCompiler {
    /// Stable identity of this backend.
    pub const ID: BackendId = BackendId::new("scalar-exact-compile");

    /// Construct the reference exact compiler.
    pub const fn new() -> Self {
        Self
    }
}

impl Backend for ReferenceExactCompiler {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(Self::ID, ExecutionTarget::PortableCpu)
    }
}

impl ExactCompiler for ReferenceExactCompiler {
    fn compile_exact(
        &self,
        _graph: &GeometryGraph,
        _root: NodeId,
        _options: &ExecutionOptions,
    ) -> GeomResult<ExactBRep> {
        // Deliberately unconditional. Narrowing this to specific families is
        // the next slice; until then every claim of exactness would be false.
        Err(GeomError::Unsupported {
            backend: Self::ID,
            operation: Operation::GraphCompilation,
        })
    }
}
