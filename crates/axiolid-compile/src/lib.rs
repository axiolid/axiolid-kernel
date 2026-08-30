#![forbid(unsafe_code)]

//! Scalar reference `GeometryCompiler`.

mod brep;
pub mod extrude;
pub mod profile;
pub mod revolve;

use axiolid_kernel::BackendId;

/// This provider's identity.
pub const BACKEND_ID: BackendId = BackendId::new("scalar-compile");

mod compiler;
pub use compiler::ScalarCompiler;
