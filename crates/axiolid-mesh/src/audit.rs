//! Deterministic structural triangle-mesh audit.
//!
//! The audit reports defects instead of rejecting dirty geometry. Open meshes can
//! still support surface distance and intersection; callers that need a watertight
//! solid must check [`MeshHealth::is_closed_two_manifold`].

use std::collections::BTreeMap;
use std::fmt;

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
    /// Two-manifold edges whose incident faces use the same directed edge.
    /// Such a mesh is closed but has inconsistent local winding, so signed
    /// enclosed-volume reduction is not structurally trustworthy.
    pub inconsistent_winding_edges: usize,
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
            && self.inconsistent_winding_edges == 0
    }
}

/// A bounded audit could not reserve its declared edge-record scratch.
#[derive(Debug)]
pub enum MeshAuditError {
    /// `3 * triangle_count` or its byte size overflowed `usize`.
    CapacityOverflow,
    /// The allocator refused the exact edge-record reservation.
    Allocation(std::collections::TryReserveError),
}

impl fmt::Display for MeshAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityOverflow => formatter.write_str("mesh audit edge count overflowed"),
            Self::Allocation(error) => write!(formatter, "mesh audit allocation failed: {error}"),
        }
    }
}

impl std::error::Error for MeshAuditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CapacityOverflow => None,
            Self::Allocation(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EdgeRecord {
    low: u64,
    high: u64,
    direction: i8,
}

/// Exact requested scratch bytes for [`try_audit_mesh`].
///
/// The bounded implementation stores at most three fixed-size edge records per
/// source triangle and sorts them in place. `None` means the count overflows.
pub const fn audit_mesh_scratch_bytes(triangle_count: usize) -> Option<usize> {
    match triangle_count.checked_mul(3) {
        Some(edges) => edges.checked_mul(std::mem::size_of::<EdgeRecord>()),
        None => None,
    }
}

#[derive(Debug, Default)]
struct EdgeSummary {
    boundary: usize,
    non_manifold: usize,
    inconsistent_winding: usize,
}

trait EdgeSink {
    fn record(&mut self, low: u64, high: u64, direction: i8);
    fn summarize(&mut self) -> EdgeSummary;
}

#[derive(Debug)]
struct VecEdgeSink {
    edges: Vec<EdgeRecord>,
}

impl VecEdgeSink {
    fn try_new(triangle_count: usize) -> Result<Self, MeshAuditError> {
        let count = triangle_count
            .checked_mul(3)
            .ok_or(MeshAuditError::CapacityOverflow)?;
        let mut edges = Vec::new();
        edges
            .try_reserve_exact(count)
            .map_err(MeshAuditError::Allocation)?;
        Ok(Self { edges })
    }
}

impl EdgeSink for VecEdgeSink {
    fn record(&mut self, low: u64, high: u64, direction: i8) {
        self.edges.push(EdgeRecord {
            low,
            high,
            direction,
        });
    }

    fn summarize(&mut self) -> EdgeSummary {
        self.edges
            .sort_unstable_by_key(|edge| (edge.low, edge.high));
        let mut summary = EdgeSummary::default();
        let mut start = 0;
        while start < self.edges.len() {
            let key = (self.edges[start].low, self.edges[start].high);
            let mut end = start + 1;
            let mut winding = i128::from(self.edges[start].direction);
            while end < self.edges.len() && (self.edges[end].low, self.edges[end].high) == key {
                winding += i128::from(self.edges[end].direction);
                end += 1;
            }
            match end - start {
                1 => summary.boundary += 1,
                2 if winding != 0 => summary.inconsistent_winding += 1,
                count if count > 2 => summary.non_manifold += 1,
                _ => {}
            }
            start = end;
        }
        summary
    }
}

#[derive(Debug, Default)]
struct MapEdgeSink {
    edges: BTreeMap<(u64, u64), (usize, i128)>,
}

impl EdgeSink for MapEdgeSink {
    fn record(&mut self, low: u64, high: u64, direction: i8) {
        let entry = self.edges.entry((low, high)).or_default();
        entry.0 = entry.0.saturating_add(1);
        entry.1 += i128::from(direction);
    }

    fn summarize(&mut self) -> EdgeSummary {
        EdgeSummary {
            boundary: self
                .edges
                .values()
                .filter(|&&(count, _)| count == 1)
                .count(),
            non_manifold: self.edges.values().filter(|&&(count, _)| count > 2).count(),
            inconsistent_winding: self
                .edges
                .values()
                .filter(|&&(count, winding)| count == 2 && winding != 0)
                .count(),
        }
    }
}

/// Audit a triangle mesh with an explicit source-unit tolerance.
///
/// A triangle is degenerate when its doubled area is at most
/// `tolerance.linear()²`; the implementation compares squared values to avoid a
/// square root. Pass [`Tolerance::ZERO`] for exact-coordinate compatibility.
///
/// This compatibility entry point falls back to the historical map-backed
/// audit if the bounded vector reservation fails. Operations with an explicit
/// memory budget should use [`try_audit_mesh`] and preflight
/// [`audit_mesh_scratch_bytes`] instead.
pub fn audit_mesh<M: TriangleMeshView + ?Sized>(mesh: &M, tolerance: Tolerance) -> MeshHealth {
    match VecEdgeSink::try_new(mesh.triangle_count()) {
        Ok(edges) => audit_with_edges(mesh, tolerance, edges),
        Err(_) => audit_with_edges(mesh, tolerance, MapEdgeSink::default()),
    }
}

/// Audit using one fallible, precomputable edge-record allocation.
///
/// Callers can refuse before allocation by comparing
/// [`audit_mesh_scratch_bytes`] with their memory budget.
pub fn try_audit_mesh<M: TriangleMeshView + ?Sized>(
    mesh: &M,
    tolerance: Tolerance,
) -> Result<MeshHealth, MeshAuditError> {
    let edges = VecEdgeSink::try_new(mesh.triangle_count())?;
    Ok(audit_with_edges(mesh, tolerance, edges))
}

fn audit_with_edges<M: TriangleMeshView + ?Sized, E: EdgeSink>(
    mesh: &M,
    tolerance: Tolerance,
    mut edges: E,
) -> MeshHealth {
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
    let squared_double_area_limit = tolerance.linear().powi(4);

    for triangle_index in 0..triangles {
        let triangle = mesh.triangle(triangle_index);
        let converted = triangle.map(|source_index| usize::try_from(source_index).ok());
        let [Some(a_index), Some(b_index), Some(c_index)] = converted else {
            for source_index in triangle {
                if usize::try_from(source_index).is_err() {
                    invalid_indices += 1;
                    first_invalid_index.get_or_insert((triangle_index, source_index));
                }
            }
            continue;
        };
        let indices = [a_index, b_index, c_index];
        let mut valid = true;
        for (corner, &index) in indices.iter().enumerate() {
            if index >= positions {
                invalid_indices += 1;
                first_invalid_index.get_or_insert((triangle_index, triangle[corner]));
                valid = false;
            }
        }
        if !valid {
            continue;
        }

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
            let direction = if left < right { 1 } else { -1 };
            edges.record(left.min(right), left.max(right), direction);
        }
    }

    let edge_summary = edges.summarize();
    MeshHealth {
        positions,
        triangles,
        usable_triangles,
        invalid_indices,
        non_finite_positions,
        degenerate_triangles,
        boundary_edges: edge_summary.boundary,
        non_manifold_edges: edge_summary.non_manifold,
        inconsistent_winding_edges: edge_summary.inconsistent_winding,
        first_invalid_index,
        first_non_finite_position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_bound_charges_three_edge_records_per_triangle() {
        assert_eq!(
            audit_mesh_scratch_bytes(7),
            7usize
                .checked_mul(3)
                .and_then(|count| count.checked_mul(std::mem::size_of::<EdgeRecord>()))
        );
        let sink = VecEdgeSink::try_new(7).expect("small bounded audit allocation");
        assert!(sink.edges.capacity() >= 21);
    }
}
