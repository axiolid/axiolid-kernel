//! Convex hull of a 3D point set, decided by certified predicates (#76).
//!
//! An incremental hull needs one geometric decision: is a point outside a
//! face's plane? `orient3d` answers exactly, so no epsilon appears here.
//! When the predicate cannot decide, the hull refuses.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::Point3;
use axiolid_guarantees::Sign;
use axiolid_mesh::TriMesh;
use axiolid_predicates::orient3d;
use std::collections::BTreeMap;

/// Convex hull of a point set as a closed, outward-oriented `TriMesh`.
///
/// Interior points are absorbed: adding a point inside the hull cannot
/// change it, which is the property the tests assert.
///
/// # Errors
///
/// Refuses non-finite coordinates, fewer than four points, and inputs whose
/// points are all collinear or all coplanar. Each is a distinct variant.
pub fn convex_hull(points: &[Point3]) -> GeomResult<TriMesh> {
    for (index, point) in points.iter().enumerate() {
        if !point.is_finite() {
            return Err(GeomError::InvalidInput(format!(
                "hull point {index} is not finite"
            )));
        }
    }
    if points.len() < 4 {
        return Err(GeomError::InvalidInput(format!(
            "a hull needs at least 4 points, got {}",
            points.len()
        )));
    }

    let seed = initial_tetrahedron(points)?;
    let mut faces = seed_faces(&seed, points);

    for (index, &point) in points.iter().enumerate() {
        if seed.contains(&index) {
            continue;
        }
        add_point(&mut faces, points, point);
    }

    Ok(assemble(&faces, points))
}

/// Four input indices spanning a non-degenerate tetrahedron.
///
/// Built in stages so each degeneracy is named: two distinct points, then a
/// third off that line, then a fourth off that plane. Failing at a stage
/// tells the caller exactly which dimension the input collapsed into.
fn initial_tetrahedron(points: &[Point3]) -> GeomResult<[usize; 4]> {
    let a = 0;
    let b = (1..points.len())
        .find(|&i| points[i] != points[a])
        .ok_or_else(|| GeomError::Degenerate("all hull points are collinear".to_owned()))?;

    // A third point off the line ab: collinear triples leave orient3d
    // undecided for every fourth point, so they must be excluded here.
    let c = (0..points.len())
        .find(|&i| i != a && i != b && !collinear(points[a], points[b], points[i]))
        .ok_or_else(|| GeomError::Degenerate("all hull points are collinear".to_owned()))?;

    let d = (0..points.len())
        .find(|&i| {
            i != a
                && i != b
                && i != c
                && orient3d(points[a], points[b], points[c], points[i]).sign() != Some(Sign::Zero)
        })
        .ok_or_else(|| GeomError::Degenerate("all hull points are coplanar".to_owned()))?;

    Ok([a, b, c, d])
}

/// Whether three points lie on one line, decided without an epsilon.
///
/// Collinearity in 3D means the triangle they span has zero area in every
/// projection. Testing `orient3d` against two independent off-plane probes
/// would still miss cases, so the cross product's exact zero is used: the
/// coordinates are the caller's own, and no arithmetic is performed beyond
/// one subtraction and one cross.
fn collinear(a: Point3, b: Point3, c: Point3) -> bool {
    (b - a).cross(c - a).length_squared() == 0.0
}

/// The seed tetrahedron's four faces, each wound to face outward.
///
/// `orient3d(a, b, c, d) == Positive` means `d` is above the plane of
/// `abc`, so `abc` viewed from outside is wound the other way. The base
/// triangle is flipped when needed so every seed face points away from the
/// fourth vertex, establishing the outward convention the incremental step
/// then preserves.
fn seed_faces(seed: &[usize; 4], points: &[Point3]) -> Vec<[usize; 3]> {
    let [a, b, c, d] = *seed;
    // Establish the outward convention explicitly rather than assuming the
    // input order supplies it. If `d` is above `abc`, then `abc` as written
    // faces INWARD, and every face built from it would too — which makes the
    // whole hull inside-out and deletes itself on the first interior point.
    let (a, b, c) =
        if orient3d(points[a], points[b], points[c], points[d]).sign() == Some(Sign::Positive) {
            (a, c, b)
        } else {
            (a, b, c)
        };
    vec![[a, b, c], [a, d, c], [a, b, d], [b, c, d]]
        .into_iter()
        .map(|face| orient_outward(face, points, d, a, b, c))
        .collect()
}

/// Wind one seed face so it faces away from the tetrahedron's interior.
///
/// The interior is represented by the centroid of the four seed vertices:
/// a face is correctly wound when the centroid is strictly behind it.
fn orient_outward(
    face: [usize; 3],
    points: &[Point3],
    d: usize,
    a: usize,
    b: usize,
    c: usize,
) -> [usize; 3] {
    let centroid = (points[a] + points[b] + points[c] + points[d]) / 4.0;
    if orient3d(points[face[0]], points[face[1]], points[face[2]], centroid).sign()
        == Some(Sign::Negative)
    {
        [face[0], face[2], face[1]]
    } else {
        face
    }
}

/// Absorb one point: delete the faces it can see, re-cover the hole.
///
/// The faces a point sees form a connected patch. Their boundary — edges
/// used by exactly one deleted face — is the horizon, and joining the point
/// to each horizon edge closes the hull again. Interior points see nothing,
/// so this is a no-op for them, which is why adding interior points cannot
/// change the result.
fn add_point(faces: &mut Vec<[usize; 3]>, points: &[Point3], point: Point3) {
    let mut visible = Vec::new();
    let mut kept = Vec::new();
    for &face in faces.iter() {
        if sees(points, face, point) {
            visible.push(face);
        } else {
            kept.push(face);
        }
    }
    if visible.is_empty() {
        return;
    }

    // Horizon edges are used once among the visible faces; edges used twice
    // are interior to the deleted patch and must not be re-covered.
    let mut usage: BTreeMap<(usize, usize), i32> = BTreeMap::new();
    for face in &visible {
        for (u, v) in edges_of(*face) {
            *usage.entry((u.min(v), u.max(v))).or_insert(0) += 1;
        }
    }

    let index = points.iter().position(|p| *p == point).unwrap_or(0);
    for face in &visible {
        for (u, v) in edges_of(*face) {
            if usage[&(u.min(v), u.max(v))] == 1 {
                kept.push([u, v, index]);
            }
        }
    }
    *faces = kept;
}

/// Whether `point` lies strictly outside `face`'s plane.
///
/// This is the whole geometric content of the algorithm, and it is exactly
/// decided. A point exactly ON the plane is NOT visible: treating it as
/// visible would delete a face and re-cover it with a zero-area triangle.
fn sees(points: &[Point3], face: [usize; 3], point: Point3) -> bool {
    orient3d(points[face[0]], points[face[1]], points[face[2]], point).sign()
        == Some(Sign::Negative)
}

/// The three directed edges of a face, in winding order.
fn edges_of(face: [usize; 3]) -> [(usize, usize); 3] {
    [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
}

/// Pack faces into a `TriMesh`, keeping only the vertices actually used.
///
/// Interior points were absorbed without contributing a vertex, so emitting
/// every input position would leave unreferenced vertices that make the mesh
/// look defective to the v0.7 diagnosis.
fn assemble(faces: &[[usize; 3]], points: &[Point3]) -> TriMesh {
    let mut remap: BTreeMap<usize, u32> = BTreeMap::new();
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for face in faces {
        for &corner in face {
            let next = u32::try_from(positions.len()).unwrap_or(u32::MAX);
            let slot = *remap.entry(corner).or_insert_with(|| {
                positions.push(points[corner]);
                next
            });
            indices.push(slot);
        }
    }
    TriMesh::new(positions, indices)
}
