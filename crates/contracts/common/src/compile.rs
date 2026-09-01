//! Complete graph compilation capability.

use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, NodeId};

use crate::{Backend, ExecutionOptions, GeomResult, OutputBound, ScratchRequirement};

/// Backend/orchestrator capable of lowering any advertised graph node to mesh.
///
/// Implementations must return [`crate::GeomError::Unsupported`] for a node they
/// do not support. They must not silently omit it or approximate exact geometry
/// unless the execution policy explicitly permits that precision.
pub trait GeometryCompiler: Backend {
    /// Scratch this compiler needs beyond the graph and the produced meshes.
    ///
    /// Defaults to [`ScratchRequirement::Unbounded`]: an unaudited compiler is
    /// treated as unbudgetable rather than silently assumed cheap.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// Outputs produced per requested root.
    ///
    /// Compilation is one mesh per root, so the destination size is known
    /// before the batch runs and workers can write disjoint slots.
    fn output_bound(&self) -> OutputBound {
        OutputBound::OneToOne
    }

    /// Compile one root.
    fn compile(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh>;

    /// Compile roots into a caller-provided buffer.
    ///
    /// This is the seam a batching implementation should override: the caller
    /// owns the destination, so a provider can reserve once from
    /// [`Self::output_bound`] and have workers write disjoint slots instead of
    /// growing a vector under a lock. `destination` is appended to, never
    /// cleared, so results can be accumulated across calls.
    ///
    /// The default is a serial loop over [`Self::compile`], which stays the
    /// only required primitive.
    fn compile_batch_into(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
        destination: &mut Vec<TriMesh>,
    ) -> GeomResult<()> {
        destination.reserve(roots.len());
        for &root in roots {
            destination.push(self.compile(graph, root, options)?);
        }
        Ok(())
    }

    /// Compile roots as a batch.
    ///
    /// Convenience over [`Self::compile_batch_into`]; overriding that instead
    /// gives both call shapes the batched behaviour.
    fn compile_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>> {
        let mut destination = Vec::with_capacity(roots.len());
        self.compile_batch_into(graph, roots, options, &mut destination)?;
        Ok(destination)
    }
}
