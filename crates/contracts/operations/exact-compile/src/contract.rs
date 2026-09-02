//! Graph-to-exact-B-rep capability.

use axiolid_brep::ExactBRep;
use axiolid_contracts::{Backend, ExecutionOptions, GeomResult, OutputBound, ScratchRequirement};
use axiolid_model::{GeometryGraph, NodeId};

/// Backend able to lower a graph root to an exact B-rep, or refuse.
///
/// The contract is deliberately narrow. An implementation returns
/// [`axiolid_contracts::GeomError::Unsupported`] naming this capability when it
/// cannot preserve exactness for a node. It must never substitute a
/// tessellation: callers wanting triangles ask
/// `axiolid_mesh_compile_contract::MeshCompiler` instead, and that request
/// carries its own explicit tolerance.
pub trait ExactCompiler: Backend {
    /// Scratch needed beyond the graph and the produced results.
    ///
    /// Defaults to unbounded: an unaudited compiler is treated as unbudgetable
    /// rather than silently assumed cheap.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// Outputs produced per requested root.
    fn output_bound(&self) -> OutputBound {
        OutputBound::OneToOne
    }

    /// Compile one root to an exact B-rep, or refuse.
    fn compile_exact(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<ExactBRep>;
}
