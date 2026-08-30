#![forbid(unsafe_code)]

//! Scalar reference `GeometryCompiler`.

mod brep;
pub mod center_line;
mod directrix;
pub mod extrude;
pub mod half_space;
pub mod loft;
pub mod profile;
pub mod revolve;
pub mod sweep;

use axiolid_kernel::BackendId;

/// This provider's identity.
pub const BACKEND_ID: BackendId = BackendId::new("scalar-compile");

mod compiler;
pub use compiler::ScalarCompiler;
