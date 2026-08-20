#![forbid(unsafe_code)]

//! Swept-solid algorithm contracts.
//!
//! Construction intent is stored in `axiolid-model::SolidOperation`; this crate
//! owns algorithms only. Backends can implement [`Sweeper`] without a format
//! adapter knowing which implementation is selected.

pub mod sweeper;

pub use sweeper::Sweeper;
