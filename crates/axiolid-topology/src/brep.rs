//! Append-only typed arenas for B-rep topology.

use crate::{
    Edge, EdgeId, Face, FaceId, Loop, LoopId, Shell, ShellId, Solid, SolidId, Vertex, VertexId,
};

/// Owned B-rep. Generic geometry handles avoid a dependency cycle with the
/// model graph that stores exact curves and surfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct BRep<G> {
    vertices: Vec<Vertex>,
    edges: Vec<Edge<G>>,
    loops: Vec<Loop<G>>,
    faces: Vec<Face<G>>,
    shells: Vec<Shell>,
    solids: Vec<Solid>,
}

impl<G> Default for BRep<G> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shells: Vec::new(),
            solids: Vec::new(),
        }
    }
}

impl<G> BRep<G> {
    /// Add a vertex and return its typed handle.
    pub fn add_vertex(&mut self, value: Vertex) -> VertexId {
        let id = VertexId::from_index(self.vertices.len());
        self.vertices.push(value);
        id
    }

    /// Add an edge and return its typed handle.
    pub fn add_edge(&mut self, value: Edge<G>) -> EdgeId {
        let id = EdgeId::from_index(self.edges.len());
        self.edges.push(value);
        id
    }

    /// Add a loop and return its typed handle.
    pub fn add_loop(&mut self, value: Loop<G>) -> LoopId {
        let id = LoopId::from_index(self.loops.len());
        self.loops.push(value);
        id
    }

    /// Add a face and return its typed handle.
    pub fn add_face(&mut self, value: Face<G>) -> FaceId {
        let id = FaceId::from_index(self.faces.len());
        self.faces.push(value);
        id
    }

    /// Add a shell and return its typed handle.
    pub fn add_shell(&mut self, value: Shell) -> ShellId {
        let id = ShellId::from_index(self.shells.len());
        self.shells.push(value);
        id
    }

    /// Add a solid and return its typed handle.
    pub fn add_solid(&mut self, value: Solid) -> SolidId {
        let id = SolidId::from_index(self.solids.len());
        self.solids.push(value);
        id
    }

    /// Vertices in stable insertion order.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Edges in stable insertion order.
    pub fn edges(&self) -> &[Edge<G>] {
        &self.edges
    }

    /// Loops in stable insertion order.
    pub fn loops(&self) -> &[Loop<G>] {
        &self.loops
    }

    /// Faces in stable insertion order.
    pub fn faces(&self) -> &[Face<G>] {
        &self.faces
    }

    /// Shells in stable insertion order.
    pub fn shells(&self) -> &[Shell] {
        &self.shells
    }

    /// Solids in stable insertion order.
    pub fn solids(&self) -> &[Solid] {
        &self.solids
    }
}
