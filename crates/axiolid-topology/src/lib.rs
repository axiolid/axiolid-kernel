#![forbid(unsafe_code)]

//! Exact B-rep topology with typed handles and caller-owned geometry links.
//!
//! The topology graph is independent of any curve/surface implementation. A
//! model can use its own handle type as `BRep<G>`, which makes this crate usable
//! by exact kernels, mesh converters, and foreign format adapters alike.

pub mod brep;
pub mod entity;
pub mod id;

pub use brep::BRep;
pub use entity::{Edge, EdgeUse, Face, FaceBound, Loop, Orientation, Shell, Solid, Vertex};
pub use id::{EdgeId, FaceId, LoopId, ShellId, SolidId, VertexId};
