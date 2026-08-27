#![forbid(unsafe_code)]

//! Backend-neutral geometry capability contracts.
//!
//! Narrow operation traits are the only capability source of truth. Concrete
//! CPU/GPU implementations live in sibling crates, preventing Cargo feature
//! unification from pulling an implementation into a format adapter.

pub mod backend;
#[cfg(feature = "mesh-boolean")]
pub mod boolean;
pub mod cancel;
pub mod capability;
pub mod certainty;
#[cfg(feature = "model")]
pub mod compile;
#[cfg(feature = "mesh-boolean")]
pub mod conformance;
pub mod error;
#[cfg(feature = "mesh-boolean")]
pub mod evidence;
pub mod execution;
#[cfg(feature = "mesh-boolean")]
pub mod solid;

pub use backend::Backend;
#[cfg(feature = "mesh-boolean")]
pub use boolean::{symmetric_difference_via_composition, MeshBoolean, MeshBooleanRegistry};
pub use cancel::{CancellationGranularity, CancellationToken};
pub use capability::{BackendDescriptor, BackendId, ExecutionTarget, Operation, Precision};
pub use certainty::{Certified, EscalationLadder, Sign};
#[cfg(feature = "model")]
pub use compile::GeometryCompiler;
pub use error::{GeomError, GeomResult};
#[cfg(feature = "mesh-boolean")]
pub use evidence::{BooleanEvidence, BooleanOutcome};
pub use execution::{
    DataResidency, Determinism, DevicePreference, ExecutionOptions, OutputBound, Parallelism,
    Residency, ScratchRequirement,
};
#[cfg(feature = "mesh-boolean")]
pub use solid::{enclosed_volume, SolidRejection, SolidRequirements};
