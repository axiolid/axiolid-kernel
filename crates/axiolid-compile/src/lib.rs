#![forbid(unsafe_code)]

//! Scalar reference `GeometryCompiler`.

mod brep;
mod directrix;

use axiolid_kernel::BackendId;

/// This provider's identity.
pub const BACKEND_ID: BackendId = BackendId::new("scalar-compile");

mod compiler;
pub use compiler::ScalarCompiler;
