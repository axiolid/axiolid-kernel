#![forbid(unsafe_code)]

//! API-neutral GPU operation adapters.
//!
//! This crate intentionally chooses no CUDA, Metal, Vulkan, or WebGPU library.
//! Concrete API crates implement a narrow operation executor and submit batches;
//! default geometry builds carry no GPU dependency or driver stack.

pub mod adapter;
pub mod device;
pub mod executor;

pub use adapter::GpuCompiler;
pub use device::{GpuDeviceDescriptor, GpuFeatures};
pub use executor::GpuGraphExecutor;

#[cfg(test)]
mod tests;
