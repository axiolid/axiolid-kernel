#![forbid(unsafe_code)]

//! Edge-collapse mesh decimation with a bounded, reported deviation (#75).
//!
//! # The contract
//!
//! A simplification that cannot state what it preserved is not a
//! simplification, it is damage. Every reduction here reports the deviation
//! it actually introduced, measured against the input surface, and refuses
//! to exceed the caller's bound.
//!
//! # What is never done
//!
//! A collapse that would invert a triangle, create a non-manifold edge, or
//! open the boundary is rejected rather than performed. Decimation must not
//! manufacture the defects `axiolid-heal` exists to find and fix.

pub mod collapse;

pub use collapse::{decimate, DecimateError, DecimateReport, DecimateTarget};
