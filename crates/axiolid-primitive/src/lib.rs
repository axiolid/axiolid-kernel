#![forbid(unsafe_code)]

//! Exact primitive solids and half-spaces.
//!
//! Values remain exact until an explicit tessellation operation. The crate has
//! no mesh or kernel dependency.

pub mod half_space;
pub mod solid;

pub use half_space::{ClipMargin, HalfSpace};
pub use solid::Primitive;
