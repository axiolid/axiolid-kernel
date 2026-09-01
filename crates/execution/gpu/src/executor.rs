//! Open graph-compiler seam implemented by concrete GPU API crates.

use axiolid_contracts::{ExecutionOptions, GeomResult};
use axiolid_mesh::TriMesh;
use axiolid_model::{GeometryGraph, NodeId};

use crate::GpuDeviceDescriptor;

/// Concrete GPU graph compiler supplied by an API-specific crate.
///
/// A CUDA executor for a workstation, a Metal executor for Apple silicon, and
/// a WebGPU executor for integrated graphics can implement this batch contract
/// without leaking vendor types into portable operation contracts.
pub trait GpuGraphExecutor: core::fmt::Debug + Send + Sync {
    /// Hardware/API facts.
    fn device(&self) -> &GpuDeviceDescriptor;

    /// Validate algorithm-specific precision, determinism, and resource policy
    /// before any work is submitted to the device.
    fn validate_options(&self, options: &ExecutionOptions) -> GeomResult<()>;

    /// Compile graph roots as one batch to amortize upload and synchronization.
    ///
    /// Return exactly one mesh per root, in input order. Honor every forwarded
    /// execution policy or return a typed `GeomError`; never silently reduce
    /// precision or determinism.
    fn compile_mesh_batch(
        &self,
        graph: &GeometryGraph,
        roots: &[NodeId],
        options: &ExecutionOptions,
    ) -> GeomResult<Vec<TriMesh>>;
}
