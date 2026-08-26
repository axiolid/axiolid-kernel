//! Deterministic structural triangle-mesh audit.
//!
//! The audit reports defects instead of rejecting dirty geometry. Open meshes can
//! still support surface distance and intersection; callers that need a watertight
//! solid must check [`MeshHealth::is_closed_two_manifold`].

use std::collections::BTreeMap;

use axiolid_core::Tolerance;

use crate::TriangleMeshView;

/// Source-neutral structural facts about one triangle mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshHealth {
    /// Number of addressable positions.
    pub positions: usize,
    /// Number of triangle records inspected.
    pub triangles: usize,
    /// Triangles with valid, finite, non-degenerate vertices.
    pub usable_triangles: usize,
    /// Number of invalid triangle-corner indices.
    pub invalid_indices: usize,
    /// Number of non-finite positions.
    pub non_finite_positions: usize,
    /// Number of triangles below the explicit area threshold.
    pub degenerate_triangles: usize,
    /// Undirected edges with exactly one usable incident triangle.
    pub boundary_edges: usize,
    /// Undirected edges with more than two usable incident triangles.
    pub non_manifold_edges: usize,
    /// First `(triangle, source_index)` which could not address a position.
    pub first_invalid_index: Option<(usize, u64)>,
    /// First non-finite position index.
    pub first_non_finite_position: Option<usize>,
}

impl MeshHealth {
    /// Whether at least one triangle can safely support surface algorithms.
    pub fn is_surface_usable(&self) -> bool {
        self.usable_triangles > 0 && self.invalid_indices == 0 && self.non_finite_positions == 0
    }

    /// Whether the usable mesh is closed and two-manifold.
    pub fn is_closed_two_manifold(&self) -> bool {
        self.is_surface_usable()
            && self.degenerate_triangles == 0
            && self.boundary_edges == 0
            && self.non_manifold_edges == 0
    }
}

/// Audit a triangle mesh with an explicit source-unit tolerance.
///
/// A triangle is degenerate when its doubled area is at most
/// `tolerance.linear()²`; the implementation compares squared values to avoid a
/// square root. Pass [`Tolerance::ZERO`] for exact-coordinate compatibility.
pub fn audit_mesh<M: TriangleMeshView + ?Sized>(mesh: &M, tolerance: Tolerance) -> MeshHealth {
    let positions = mesh.position_count();
    let triangles = mesh.triangle_count();
    let first_non_finite_position = (0..positions).find(|&index| !mesh.position(index).is_finite());
    let non_finite_positions = (0..positions)
        .filter(|&index| !mesh.position(index).is_finite())
        .count();
    let mut invalid_indices = 0;
    let mut first_invalid_index = None;
    let mut degenerate_triangles = 0;
    let mut usable_triangles = 0;
    let mut edges = BTreeMap::<(u64, u64), usize>::new();
    let squared_double_area_limit = tolerance.linear().powi(4);

    for triangle_index in 0..triangles {
        let triangle = mesh.triangle(triangle_index);
        let mut valid = true;
        for source_index in triangle {
            let in_bounds = usize::try_from(source_index)
                .ok()
                .is_some_and(|index| index < positions);
            if !in_bounds {
                invalid_indices += 1;
                first_invalid_index.get_or_insert((triangle_index, source_index));
                valid = false;
            }
        }
        if !valid {
            continue;
        }
        let indices = triangle.map(|index| index as usize);
        let [a, b, c] = indices.map(|index| mesh.position(index));
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            continue;
        }
        let squared_double_area = (b - a).cross(c - a).length_squared();
        if squared_double_area <= squared_double_area_limit {
            degenerate_triangles += 1;
            continue;
        }
        usable_triangles += 1;
        for (left, right) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            *edges.entry((left.min(right), left.max(right))).or_default() += 1;
        }
    }

    MeshHealth {
        positions,
        triangles,
        usable_triangles,
        invalid_indices,
        non_finite_positions,
        degenerate_triangles,
        boundary_edges: edges.values().filter(|&&count| count == 1).count(),
        non_manifold_edges: edges.values().filter(|&&count| count > 2).count(),
        first_invalid_index,
        first_non_finite_position,
    }
}
