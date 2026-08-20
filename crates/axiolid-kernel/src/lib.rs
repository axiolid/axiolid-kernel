#![forbid(unsafe_code)]

//! Backend-neutral geometry capability contracts.
//!
//! Narrow operation traits are the only capability source of truth. Concrete
//! CPU/GPU implementations live in sibling crates, preventing Cargo feature
//! unification from pulling an implementation into a format adapter.

pub mod backend;
#[cfg(feature = "mesh-boolean")]
pub mod boolean;
pub mod capability;
pub mod certainty;
#[cfg(feature = "model")]
pub mod compile;
pub mod error;
pub mod execution;

pub use backend::Backend;
#[cfg(feature = "mesh-boolean")]
pub use boolean::{MeshBoolean, MeshBooleanRegistry};
pub use capability::{BackendDescriptor, BackendId, ExecutionTarget, Operation, Precision};
pub use certainty::{Certified, EscalationLadder, Sign};
#[cfg(feature = "model")]
pub use compile::GeometryCompiler;
pub use error::{GeomError, GeomResult};
pub use execution::{
    DataResidency, Determinism, DevicePreference, ExecutionOptions, OutputBound, Parallelism,
    Residency, ScratchRequirement,
};
