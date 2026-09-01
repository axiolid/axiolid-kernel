#![forbid(unsafe_code)]

//! `boolmesh`-backed [`axiolid_kernel::MeshBoolean`] provider (ADR 0014).

mod convert;
mod grouping;
mod provider;

pub use provider::BoolmeshBoolean;
