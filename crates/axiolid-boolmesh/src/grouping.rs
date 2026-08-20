//! Disjoint-cutter grouping for `subtract_many`.
//!
//! The sequential default subtracts tools one at a time, so the subject is
//! rebuilt on every step: N booleans against a mesh that keeps growing. The
//! observation that makes batching possible is that
//!
//! ```text
//! (S \ A) \ B  ==  S \ (A union B)
//! ```
//!
//! and when `A` and `B` are disjoint, `A union B` is just their concatenation
//! -- no boolean needed to build it. So a set of mutually disjoint cutters can
//! be fused into ONE tool mesh and removed with a single boolean.
//!
//! Grouping is therefore a graph-colouring problem on the overlap graph:
//! partition the tools into as few groups as possible such that no two tools
//! in a group overlap. Optimal colouring is NP-hard, so a greedy first-fit is
//! used; the result only needs to be good, not optimal, and any partition is
//! CORRECT because each group is verified disjoint before fusing.

use axiolid_core::Aabb;
use axiolid_mesh::TriMesh;

/// Concatenate meshes into one, rebasing indices.
///
/// Sound only for meshes with disjoint interiors: the result is a single mesh
/// with several connected components, which is a valid manifold exactly when
/// the components do not touch. The caller guarantees that.
pub(crate) fn fuse(meshes: &[&TriMesh]) -> TriMesh {
    let vertices = meshes.iter().map(|m| m.positions.len()).sum();
    let triangles = meshes.iter().map(|m| m.indices.len()).sum();
    let mut positions = Vec::with_capacity(vertices);
    let mut indices = Vec::with_capacity(triangles);
    for mesh in meshes {
        let offset = positions.len() as u32;
        positions.extend_from_slice(&mesh.positions);
        indices.extend(mesh.indices.iter().map(|&i| i + offset));
    }
    TriMesh::new(positions, indices)
}

/// Partition tool indices into groups whose bounds are mutually disjoint.
///
/// Greedy first-fit coloring over the AABB overlap graph. Tools are visited in
/// input order so the partition is deterministic: the same input always yields
/// the same groups, which keeps the batch path's output reproducible.
///
/// Bounds are a CONSERVATIVE overlap test: two disjoint solids can have
/// overlapping boxes, in which case they are needlessly separated. That costs
/// a group, never correctness. The converse -- disjoint boxes but overlapping
/// solids -- is impossible, which is what makes the fused union sound.
pub(crate) fn disjoint_groups(bounds: &[Aabb]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    // Bounds per group, kept alongside so membership tests do not re-walk
    // meshes. Index-parallel with `groups`.
    let mut group_bounds: Vec<Vec<Aabb>> = Vec::new();

    'tool: for (index, bound) in bounds.iter().enumerate() {
        for (group, members) in group_bounds.iter_mut().enumerate() {
            if members.iter().all(|existing| !existing.intersects(bound)) {
                groups[group].push(index);
                members.push(*bound);
                continue 'tool;
            }
        }
        groups.push(vec![index]);
        group_bounds.push(vec![*bound]);
    }
    groups
}
