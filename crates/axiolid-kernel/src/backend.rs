//! Open backend contract. Implementations live in separate crates.

use crate::BackendDescriptor;

/// Runtime geometry backend metadata.
///
/// The trait is intentionally unsealed: a downstream x86, ARM, CUDA, Metal,
/// WebGPU, or domain-specific accelerator crate owns its backend type and can
/// implement this trait without modifying Axiolid.
pub trait Backend: core::fmt::Debug + Send + Sync {
    /// Runtime descriptor and capability inventory.
    fn descriptor(&self) -> BackendDescriptor;
}
