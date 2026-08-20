#![forbid(unsafe_code)]

//! Metric-property contracts.
//!
//! Algorithms are generic over representation and return structured failures;
//! an open shell must not silently report a plausible volume.

pub mod measure;
pub mod properties;

pub use measure::Measure;
pub use properties::MassProperties;
