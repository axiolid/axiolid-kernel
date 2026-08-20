//! B-rep entities. `G` is a caller-chosen curve/surface geometry handle.

use axiolid_core::Point3;

use crate::{EdgeId, FaceId, LoopId, ShellId, VertexId};

/// Topological orientation relative to the underlying geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Same parameter direction/normal.
    Forward,
    /// Reversed parameter direction/normal.
    Reversed,
}

/// Vertex with an explicit model-space position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position.
    pub position: Point3,
}

/// Edge bounded by two vertices and optionally supported by exact curve data.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge<G> {
    /// Start vertex.
    pub start: VertexId,
    /// End vertex.
    pub end: VertexId,
    /// Exact support curve handle; absent for a straight topological edge whose
    /// endpoints are sufficient.
    pub curve: Option<G>,
}

/// One oriented use of an edge in a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeUse {
    /// Referenced edge.
    pub edge: EdgeId,
    /// Traversal direction.
    pub orientation: Orientation,
}

/// Closed boundary wire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Loop {
    /// Consecutive oriented edges.
    pub edges: Vec<EdgeUse>,
}

/// One oriented loop use on a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceBound {
    /// Referenced loop.
    pub loop_id: LoopId,
    /// Whether this bound has the same orientation as the face.
    pub orientation: Orientation,
    /// Whether this is the outer bound.
    pub outer: bool,
}

/// Face supported by an exact surface and bounded by loops.
#[derive(Debug, Clone, PartialEq)]
pub struct Face<G> {
    /// Exact support surface handle. Planar polygonal faces may omit it.
    pub surface: Option<G>,
    /// Outer and inner bounds.
    pub bounds: Vec<FaceBound>,
    /// Orientation relative to the support surface normal.
    pub orientation: Orientation,
}

/// Connected collection of oriented faces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shell {
    /// Face handles.
    pub faces: Vec<(FaceId, Orientation)>,
    /// Whether the source asserts closure.
    pub closed: bool,
}

/// Solid with one outer shell and optional void shells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solid {
    /// Outer shell.
    pub outer: ShellId,
    /// Interior void shells.
    pub voids: Vec<ShellId>,
}
