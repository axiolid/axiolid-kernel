#![forbid(unsafe_code)]

//! Solid generation: turning exact profiles and paths into meshes.
//!
//! # Why this is its own crate
//!
//! Everything here answers one question: given an exact profile and a path,
//! what solid does that denote? Extrusion, revolution, the sweep families,
//! lofting and half-space clipping are all the same problem with different
//! path kinds, and they share one stitching implementation so that winding,
//! hole orientation and cap pairing are decided once.
//!
//! None of that needs an operation graph. These functions take geometry and
//! return geometry; they do not walk a DAG, cache results, resolve node
//! references, or dispatch to a backend. That is why they live here and not
//! in `axiolid-compile`, which does all four.
//!
//! The split matters beyond tidiness. A caller that wants a swept solid --
//! a CAD front end, a test, a future exact B-rep generator -- should not have
//! to construct a `SolidOperation` graph and run a compiler to get one. Under
//! the old layout that was the only way to reach this code.
//!
//! # What this crate does not do
//!
//! It produces meshes, not exact B-rep. Per [ADR 0020](
//! https://axiolid.github.io/axiolid-kernel/adr/0020-exact-brep-kernel-model)
//! that is a current limitation, not the intended end state: these generators
//! are the natural place for exact swept-surface output to appear once the
//! representation exists, and the crate is named for the operation rather
//! than for its present output type so that change does not require a rename.

use axiolid_kernel::BackendId;

/// Identity these generators report in diagnostics.
///
/// Distinct from the compiler's: a failure raised while building a swept
/// solid comes from this crate, and attributing it to `axiolid-compile`
/// would send a reader to the wrong place. Sweeps already reported a
/// separate identity before the split; this makes every generator
/// consistent with that.
pub const BACKEND_ID: BackendId = BackendId::new("scalar-generate");

pub mod center_line;
pub mod extrude;
pub mod half_space;
pub mod loft;
pub mod profile;
pub mod revolve;
pub mod sweep;
