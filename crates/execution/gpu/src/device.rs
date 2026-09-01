//! API-neutral GPU device description.

use axiolid_contracts::BackendId;

/// Relevant compute features without exposing CUDA, Metal, Vulkan, or WebGPU
/// types in the public kernel contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuFeatures {
    /// Native binary64 shader/compute arithmetic.
    pub float64: bool,
    /// Subgroup/warp operations.
    pub subgroups: bool,
    /// Host-visible unified memory.
    pub unified_memory: bool,
    /// Maximum workgroup size reported by the concrete API.
    pub max_workgroup_size: u32,
}

/// Runtime GPU identity for diagnostics and selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDeviceDescriptor {
    /// Stable operation-provider/device identifier.
    pub id: BackendId,
    /// Human-readable adapter name.
    pub name: String,
    /// Driver/API name, e.g. CUDA, Metal, Vulkan, or WebGPU.
    pub api: String,
    /// Relevant features.
    pub features: GpuFeatures,
}
