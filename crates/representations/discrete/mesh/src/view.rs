//! Borrowed mesh interoperability seam.

use axiolid_core::Point3;

use crate::TriMesh;

/// Read-only triangle mesh view.
///
/// A foreign kernel can implement this for its native mesh and call algorithms
/// without first adopting Axiolid's owned container.
pub trait MeshView {
    /// Position buffer.
    fn positions(&self) -> &[Point3];
    /// Triangle corner index buffer.
    fn indices(&self) -> &[u32];
}

impl MeshView for TriMesh {
    fn positions(&self) -> &[Point3] {
        &self.positions
    }

    fn indices(&self) -> &[u32] {
        &self.indices
    }
}

/// Read-only triangle mesh with arbitrary caller index storage.
///
/// This is the validation seam for foreign meshes. Implementations must return
/// a finite, stable position for every `index < position_count()`; triangle
/// indices may be invalid and are reported by validation rather than trusted.
pub trait TriangleMeshView {
    /// Number of addressable positions.
    fn position_count(&self) -> usize;
    /// Position at a valid index.
    fn position(&self, index: usize) -> Point3;
    /// Number of complete triangle records.
    fn triangle_count(&self) -> usize;
    /// Three caller-provided position indices for one triangle.
    fn triangle(&self, index: usize) -> [u64; 3];
}

impl TriangleMeshView for TriMesh {
    fn position_count(&self) -> usize {
        self.positions.len()
    }

    fn position(&self, index: usize) -> Point3 {
        self.positions[index]
    }

    fn triangle_count(&self) -> usize {
        self.triangle_count()
    }

    fn triangle(&self, index: usize) -> [u64; 3] {
        let triangle: [u32; 3] = self.indices[index * 3..index * 3 + 3]
            .try_into()
            .expect("triangle_count only exposes complete triples");
        triangle.map(u64::from)
    }
}
