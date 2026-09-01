#![forbid(unsafe_code)]
//! Shared admissibility requirements for mesh-valued operations.

mod solid;

pub use solid::{enclosed_volume, SolidRejection, SolidRequirements};
