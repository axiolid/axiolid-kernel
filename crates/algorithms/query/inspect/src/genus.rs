//! Topological genus from the Euler characteristic.
//!
//! For a closed orientable surface, V - E + F = 2 - 2g. The formula is only
//! meaningful on a closed two-manifold, so this refuses anything else
//! rather than returning a number the caller would have no way to distrust.

use axiolid_mesh::TriMesh;
use std::collections::BTreeSet;
use thiserror::Error;

/// Why a genus could not be computed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenusError {
    /// The mesh is not a closed two-manifold, so Euler's formula does not apply.
    #[error("genus requires a closed two-manifold; found {boundary} boundary and {non_manifold} non-manifold edges")]
    NotClosedManifold {
        /// Edges used by exactly one triangle.
        boundary: usize,
        /// Edges used by more than two triangles.
        non_manifold: usize,
    },
    /// The Euler characteristic was odd, so no integer genus exists.
    #[error("Euler characteristic {characteristic} is odd; the surface is not orientable")]
    NotOrientable {
        /// The computed characteristic.
        characteristic: i64,
    },
}

/// Genus of a closed orientable triangle mesh.
///
/// A sphere or cube is 0, a torus 1, a double torus 2.
///
/// # Errors
///
/// Refuses a mesh with boundary or non-manifold edges: Euler's formula
/// assumes a closed two-manifold, and applying it anyway would produce a
/// plausible-looking integer with no meaning.
pub fn genus(mesh: &TriMesh) -> Result<u32, GenusError> {
    let faces = mesh.indices.len() / 3;
    let mut edge_uses: std::collections::BTreeMap<(u32, u32), usize> =
        std::collections::BTreeMap::new();
    let mut vertices = BTreeSet::new();

    for triangle in mesh.indices.chunks_exact(3) {
        for corner in triangle {
            vertices.insert(*corner);
        }
        for i in 0..3 {
            let a = triangle[i];
            let b = triangle[(i + 1) % 3];
            // Undirected key: the same edge from either side must collide.
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_uses.entry(key).or_insert(0) += 1;
        }
    }

    let boundary = edge_uses.values().filter(|uses| **uses == 1).count();
    let non_manifold = edge_uses.values().filter(|uses| **uses > 2).count();
    if boundary > 0 || non_manifold > 0 {
        return Err(GenusError::NotClosedManifold {
            boundary,
            non_manifold,
        });
    }

    // Only vertices actually referenced by a triangle count: an unused
    // position is stray data, not part of the surface.
    let v = vertices.len() as i64;
    let e = edge_uses.len() as i64;
    let f = faces as i64;
    let characteristic = v - e + f;

    // chi = 2 - 2g, so g = (2 - chi) / 2. An odd characteristic means the
    // input is not a closed orientable surface after all.
    let doubled = 2 - characteristic;
    if doubled % 2 != 0 {
        return Err(GenusError::NotOrientable { characteristic });
    }
    Ok(u32::try_from(doubled / 2).unwrap_or(0))
}
