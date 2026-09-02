//! Graph tessellation capability.

use axiolid_model::{GeometryGraph, NodeId};

use crate::{TessellatedMesh, TessellationOptions};

/// Convert exact graph nodes to watertight triangle meshes.
pub trait Tessellator: core::fmt::Debug + Send + Sync {
    /// Structured failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Tessellate one node.
    fn tessellate(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &TessellationOptions,
    ) -> Result<TessellatedMesh, Self::Error>;

    /// Tessellate many roots. Implementations should override for parallel/GPU
    /// batching and must preserve input order.
    fn tessellate_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &TessellationOptions,
    ) -> Result<Vec<TessellatedMesh>, Self::Error> {
        roots
            .iter()
            .map(|&root| self.tessellate(graph, root, options))
            .collect()
    }
}
