//! Mesh-level proximity: distance with witnesses, and proximity components.
//!
//! # What the kernel decides and what it does not
//!
//! The kernel owns the distance, the witnesses, and the component
//! decomposition. It does not own the verdict: whether a separation is a
//! clash, a tolerance violation, or acceptable is caller policy.
//!
//! # Surface contact is not solid penetration
//!
//! These routines measure SURFACE separation. Two solids whose boundaries
//! touch report distance zero; so do two that interpenetrate deeply, because
//! their surfaces cross. A contact area and an interpenetration depth are
//! different measurements and must not be conflated behind one number, so
//! neither is inferred here: [`MeshDistance::surfaces_cross`] reports the fact
//! and leaves the interpretation to the caller.

use axiolid_core::Point3;
use axiolid_mesh::TriangleMeshView;

use crate::proximity::{closest_points_on_triangles, ClosestPoints3, ProximityError};

/// Why a mesh proximity query could not be answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MeshProximityError {
    /// A mesh had no usable triangles.
    ///
    /// Distance to nothing is undefined; returning infinity would let a
    /// caller read an empty mesh as "very far away" rather than "no input".
    EmptyMesh,
    /// A triangle referenced a position the mesh does not have.
    IndexOutOfRange,
    /// The threshold was negative or not finite.
    InvalidThreshold,
    /// A primitive query rejected its input.
    Primitive(ProximityError),
}

impl From<ProximityError> for MeshProximityError {
    fn from(error: ProximityError) -> Self {
        Self::Primitive(error)
    }
}

/// The separation between two meshes and the witness pair realising it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshDistance {
    /// Witness on the first mesh.
    pub point_a: Point3,
    /// Witness on the second mesh.
    pub point_b: Point3,
    /// Squared separation of the witnesses.
    ///
    /// Squared to stay exact for the comparisons callers make with it; take
    /// the square root only for display.
    pub distance_squared: f64,
    /// Index of the first mesh triangle carrying the witness.
    pub triangle_a: usize,
    /// Index of the second mesh triangle carrying the witness.
    pub triangle_b: usize,
    /// The surfaces meet or cross at the witness.
    ///
    /// True exactly when the separation is zero. This is a SURFACE fact and
    /// says nothing about penetration depth or contact area: grazing contact
    /// and deep interpenetration both report zero here.
    pub surfaces_cross: bool,
}

/// A region where two meshes approach within a threshold.
#[derive(Debug, Clone, PartialEq)]
pub struct ProximityComponent {
    /// The closest approach within this component.
    pub witness: MeshDistance,
    /// First-mesh triangles participating, ascending.
    pub triangles_a: Vec<usize>,
    /// Second-mesh triangles participating, ascending.
    pub triangles_b: Vec<usize>,
}

/// Distance between two meshes, with the witness pair realising it.
///
/// Exhaustive over triangle pairs: correct by construction, and the reference
/// the broad-phase-accelerated path is checked against.
///
/// Ties break on the lowest `(triangle_a, triangle_b)` pair, so the witness is
/// reproducible rather than dependent on iteration order.
pub fn mesh_distance<A, B>(first: &A, second: &B) -> Result<MeshDistance, MeshProximityError>
where
    A: TriangleMeshView + ?Sized,
    B: TriangleMeshView + ?Sized,
{
    let (triangles_a, _) = collect(first)?;
    let (triangles_b, _) = collect(second)?;

    let mut best: Option<MeshDistance> = None;
    for (index_a, tri_a) in triangles_a.iter().enumerate() {
        for (index_b, tri_b) in triangles_b.iter().enumerate() {
            let pair = closest_points_on_triangles(*tri_a, *tri_b)?;
            let candidate = to_distance(pair, index_a, index_b);
            // Strictly less, so the first pair in index order wins a tie.
            if best.is_none_or(|current| candidate.distance_squared < current.distance_squared) {
                best = Some(candidate);
            }
        }
    }

    best.ok_or(MeshProximityError::EmptyMesh)
}

fn to_distance(pair: ClosestPoints3, triangle_a: usize, triangle_b: usize) -> MeshDistance {
    MeshDistance {
        point_a: pair.point_a,
        point_b: pair.point_b,
        distance_squared: pair.distance_squared,
        triangle_a,
        triangle_b,
        surfaces_cross: pair.distance_squared == 0.0,
    }
}

type Collected = (Vec<[Point3; 3]>, Vec<[usize; 3]>);

fn collect<M: TriangleMeshView + ?Sized>(mesh: &M) -> Result<Collected, MeshProximityError> {
    let positions = mesh.position_count();
    let mut triangles = Vec::with_capacity(mesh.triangle_count());
    let mut corner_indices = Vec::with_capacity(mesh.triangle_count());
    for index in 0..mesh.triangle_count() {
        let corners = mesh.triangle(index);
        let mut points = [Point3::ZERO; 3];
        let mut corner_ids = [0usize; 3];
        for (slot, corner) in corners.iter().enumerate() {
            let corner =
                usize::try_from(*corner).map_err(|_| MeshProximityError::IndexOutOfRange)?;
            if corner >= positions {
                return Err(MeshProximityError::IndexOutOfRange);
            }
            points[slot] = mesh.position(corner);
            corner_ids[slot] = corner;
        }
        triangles.push(points);
        corner_indices.push(corner_ids);
    }
    if triangles.is_empty() {
        return Err(MeshProximityError::EmptyMesh);
    }
    Ok((triangles, corner_indices))
}

/// Disjoint regions where two meshes approach within `threshold`.
///
/// Two close pairs belong to the same component when they share a triangle on
/// either mesh; connectivity is transitive through that relation. This groups
/// one physical approach into one component instead of reporting every
/// triangle pair separately.
///
/// Components are ordered by closest approach, nearest first, with ties broken
/// on the witness triangle indices so the order is reproducible.
pub fn proximity_components<A, B>(
    first: &A,
    second: &B,
    threshold: f64,
) -> Result<Vec<ProximityComponent>, MeshProximityError>
where
    A: TriangleMeshView + ?Sized,
    B: TriangleMeshView + ?Sized,
{
    if !threshold.is_finite() || threshold < 0.0 {
        return Err(MeshProximityError::InvalidThreshold);
    }
    let (triangles_a, corners_a) = collect(first)?;
    let (triangles_b, corners_b) = collect(second)?;
    let limit = threshold * threshold;

    // Close pairs first; the union-find below groups them.
    let mut close = Vec::new();
    for (index_a, tri_a) in triangles_a.iter().enumerate() {
        for (index_b, tri_b) in triangles_b.iter().enumerate() {
            let pair = closest_points_on_triangles(*tri_a, *tri_b)?;
            if pair.distance_squared <= limit {
                close.push(to_distance(pair, index_a, index_b));
            }
        }
    }
    if close.is_empty() {
        return Ok(Vec::new());
    }

    // Two close pairs belong to the same approach when their triangles are
    // CONNECTED on both meshes: same triangle, or sharing a vertex with it.
    //
    // Weaker rules were tried and are wrong. Sharing a triangle on either mesh
    // alone merges everything one long triangle touches, so a bar spanning two
    // separated squares reports a single approach. Witness distance alone
    // splits one approach into several, because witnesses on adjacent
    // triangles of the same flat face sit a whole triangle apart, which says
    // nothing about the approach's extent. Adjacency is the relation that
    // actually tracks "same piece of surface".
    let mut parent: Vec<usize> = (0..close.len()).collect();
    for i in 0..close.len() {
        for j in i + 1..close.len() {
            let linked_a = adjacent(&corners_a, close[i].triangle_a, close[j].triangle_a);
            let linked_b = adjacent(&corners_b, close[i].triangle_b, close[j].triangle_b);
            if linked_a && linked_b {
                union(&mut parent, i, j);
            }
        }
    }

    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for index in 0..close.len() {
        let root = find(&mut parent, index);
        match groups.iter_mut().find(|(key, _)| *key == root) {
            Some((_, members)) => members.push(index),
            None => groups.push((root, vec![index])),
        }
    }

    let mut components: Vec<ProximityComponent> = groups
        .into_iter()
        .map(|(_, members)| build_component(&close, &members))
        .collect();
    components.sort_by(|a, b| {
        a.witness
            .distance_squared
            .total_cmp(&b.witness.distance_squared)
            .then(a.witness.triangle_a.cmp(&b.witness.triangle_a))
            .then(a.witness.triangle_b.cmp(&b.witness.triangle_b))
    });
    Ok(components)
}

fn build_component(close: &[MeshDistance], members: &[usize]) -> ProximityComponent {
    let mut witness = close[members[0]];
    let mut triangles_a = Vec::new();
    let mut triangles_b = Vec::new();
    for &index in members {
        let entry = close[index];
        if entry.distance_squared < witness.distance_squared {
            witness = entry;
        }
        triangles_a.push(entry.triangle_a);
        triangles_b.push(entry.triangle_b);
    }
    triangles_a.sort_unstable();
    triangles_a.dedup();
    triangles_b.sort_unstable();
    triangles_b.dedup();
    ProximityComponent {
        witness,
        triangles_a,
        triangles_b,
    }
}

fn find(parent: &mut [usize], mut node: usize) -> usize {
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let (root_a, root_b) = (find(parent, a), find(parent, b));
    if root_a != root_b {
        parent[root_b] = root_a;
    }
}

/// Two triangles are the same or share at least one vertex.
fn adjacent(corners: &[[usize; 3]], first: usize, second: usize) -> bool {
    first == second
        || corners[first]
            .iter()
            .any(|a| corners[second].iter().any(|b| a == b))
}
