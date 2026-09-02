//! Graph-to-exact-B-rep capability.

use axiolid_brep::ExactBRep;
use axiolid_contracts::{Backend, ExecutionOptions, GeomResult, OutputBound, ScratchRequirement};
use axiolid_model::{GeometryGraph, NodeId};

/// Backend able to lower graph roots to exact B-reps, or refuse.
///
/// The contract is deliberately narrow. An implementation returns
/// [`axiolid_contracts::GeomError::UnsupportedInput`] naming the unsupported
/// node family when it cannot preserve exactness. It must never substitute a
/// tessellation: callers wanting triangles ask
/// `axiolid_mesh_compile_contract::MeshCompiler` instead.
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

    /// Compile roots into a caller-provided buffer.
    ///
    /// Results append in root order. Implementations override this seam to
    /// share a per-request exact-result cache across roots.
    fn compile_exact_batch_into(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
        destination: &mut Vec<ExactBRep>,
    ) -> GeomResult<()> {
        destination.reserve(roots.len());
        for &root in roots {
            destination.push(self.compile_exact(graph, root, options)?);
        }
        Ok(())
    }

    /// Compile roots as an ordered exact-result batch.
    fn compile_exact_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<ExactBRep>> {
        let mut destination = Vec::with_capacity(roots.len());
        self.compile_exact_batch_into(graph, roots, options, &mut destination)?;
        Ok(destination)
    }
}
