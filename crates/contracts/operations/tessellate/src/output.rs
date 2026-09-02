//! Discrete geometry plus the tolerance that produced it.

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;

/// A triangle mesh with immutable approximation provenance.
///
/// The wrapper prevents a generated mesh from becoming tolerance-free currency
/// between operations. Callers can still recover the representation explicitly
/// with [`Self::into_mesh`].
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedMesh {
    mesh: TriMesh,
    tolerance: Tolerance,
}

impl TessellatedMesh {
    /// Bind a produced mesh to the tolerance used to create it.
    #[must_use]
    pub const fn new(mesh: TriMesh, tolerance: Tolerance) -> Self {
        Self { mesh, tolerance }
    }

    /// Borrow the triangle representation.
    #[must_use]
    pub const fn mesh(&self) -> &TriMesh {
        &self.mesh
    }

    /// Approximation tolerance used to produce the mesh.
    #[must_use]
    pub const fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Consume the artifact and recover its triangle representation.
    #[must_use]
    pub fn into_mesh(self) -> TriMesh {
        self.mesh
    }
}
