//! Graph tessellation capability.

use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, NodeId};

use crate::TessellationOptions;

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
    ) -> Result<TriMesh, Self::Error>;

    /// Tessellate many roots. Implementations should override for parallel/GPU
    /// batching and must preserve input order.
    fn tessellate_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &TessellationOptions,
    ) -> Result<Vec<TriMesh>, Self::Error> {
        roots
            .iter()
            .map(|&root| self.tessellate(graph, root, options))
            .collect()
    }
}
