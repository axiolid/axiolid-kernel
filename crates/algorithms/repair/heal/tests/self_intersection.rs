//! Self-intersection detection, and what must NOT count as one (#73).
//!
//! The failure mode this guards against is over-reporting. In a closed mesh
//! every triangle touches its neighbours, so an implementation that treats
//! contact as intersection reports every valid solid as broken. The adjacency
//! cases below are therefore the load-bearing tests, not the crossing one.

use axiolid_heal::{self_intersections, self_intersections_brute_force};
use axiolid_mesh::TriMesh;

/// Two triangles sharing edge (1,2), forming a valid strip.
fn shared_edge() -> TriMesh {
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [1.0, 0.0, 0.0].into(),
        [0.0, 1.0, 0.0].into(),
        [1.0, 1.0, 0.0].into(),
    ];
    TriMesh::new(positions, vec![0, 1, 2, 1, 3, 2])
}

/// Two triangles meeting only at vertex 0.
fn shared_vertex() -> TriMesh {
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [1.0, 0.0, 0.0].into(),
        [0.0, 1.0, 0.0].into(),
        [-1.0, 0.0, 0.0].into(),
        [0.0, -1.0, 0.0].into(),
    ];
    TriMesh::new(positions, vec![0, 1, 2, 0, 3, 4])
}

/// Two triangles that genuinely cross: a vertical blade through a flat one.
fn crossing() -> TriMesh {
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [2.0, 0.0, 0.0].into(),
        [0.0, 2.0, 0.0].into(),
        [0.5, 0.5, -1.0].into(),
        [0.5, 0.5, 1.0].into(),
        [1.5, 1.5, 0.0].into(),
    ];
    TriMesh::new(positions, vec![0, 1, 2, 3, 4, 5])
}

#[test]
fn triangles_sharing_an_edge_do_not_intersect() {
    let mesh = shared_edge();
    assert!(
        self_intersections(&mesh).is_empty(),
        "adjacent triangles touch along their shared edge; that is what a mesh IS"
    );
}

#[test]
fn triangles_sharing_only_a_vertex_do_not_intersect() {
    let mesh = shared_vertex();
    assert!(
        self_intersections(&mesh).is_empty(),
        "a shared corner is contact, not penetration"
    );
}

#[test]
fn a_crossing_pair_is_reported_exactly_once() {
    let mesh = crossing();
    let found = self_intersections(&mesh);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one crossing pair: {found:?}"
    );
    assert_eq!(
        (found[0].first, found[0].second),
        (0, 1),
        "the reported pair must name WHICH triangles cross"
    );
}

#[test]
fn a_closed_box_has_no_self_intersections() {
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [1.0, 0.0, 0.0].into(),
        [1.0, 1.0, 0.0].into(),
        [0.0, 1.0, 0.0].into(),
        [0.0, 0.0, 1.0].into(),
        [1.0, 0.0, 1.0].into(),
        [1.0, 1.0, 1.0].into(),
        [0.0, 1.0, 1.0].into(),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    let mesh = TriMesh::new(positions, indices);
    assert!(
        self_intersections(&mesh).is_empty(),
        "a unit cube is the canonical valid solid"
    );
}

/// The index must not change the answer, only the cost.
///
/// Broad-phase acceleration is an optimisation. If the BVH and the exhaustive
/// pairwise scan ever disagree, the index has hidden or invented a pair, and
/// the accelerated answer is wrong in a way no single-case test would reveal.
#[test]
fn the_index_and_brute_force_agree() {
    for (name, mesh) in [
        ("shared edge", shared_edge()),
        ("shared vertex", shared_vertex()),
        ("crossing", crossing()),
    ] {
        let indexed = self_intersections(&mesh);
        let exhaustive = self_intersections_brute_force(&mesh);
        assert_eq!(
            indexed, exhaustive,
            "{name}: the spatial index disagreed with the exhaustive scan"
        );
    }
}

/// A self-intersecting closed mesh is still detected.
///
/// The cube fix made the test stricter, so it must be shown that strictness
/// did not silence real detections: two overlapping boxes in one mesh are a
/// closed, structurally valid, self-intersecting solid.
#[test]
fn two_overlapping_boxes_in_one_mesh_are_detected() {
    let mut positions: Vec<axiolid_core::Point3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for offset in [0.0, 0.5] {
        let base = positions.len() as u32;
        let corners: [axiolid_core::Point3; 8] = [
            [offset, 0.0, 0.0].into(),
            [offset + 1.0, 0.0, 0.0].into(),
            [offset + 1.0, 1.0, 0.0].into(),
            [offset, 1.0, 0.0].into(),
            [offset, 0.0, 1.0].into(),
            [offset + 1.0, 0.0, 1.0].into(),
            [offset + 1.0, 1.0, 1.0].into(),
            [offset, 1.0, 1.0].into(),
        ];
        positions.extend(corners);
        indices.extend(
            [
                0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2,
                7, 6, 3, 0, 4, 3, 4, 7,
            ]
            .iter()
            .map(|i| i + base),
        );
    }
    let mesh = TriMesh::new(positions, indices);
    let found = self_intersections(&mesh);
    assert!(
        !found.is_empty(),
        "two boxes overlapping by half their width must self-intersect"
    );
    assert_eq!(
        found,
        self_intersections_brute_force(&mesh),
        "index and exhaustive scan must agree on the overlapping case too"
    );
}

/// Adjacency is decided on indices, not geometry.
///
/// A folded pair sharing an edge, where one triangle's far vertex sits on the
/// other's plane, is a configuration the interval test alone
/// reports as touching. Two triangles of a real mesh are allowed to touch
/// along the edge they share, so the index-based guard has to reject it
/// before any arithmetic runs.
#[test]
fn a_shared_edge_is_not_an_intersection_even_when_geometry_touches() {
    // Triangle A: (0,1,2) in the z = 0 plane.
    // Triangle B: shares edge (1,2), and its third vertex lies IN A's plane,
    // folded back so it overlaps A's interior region.
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [2.0, 0.0, 0.0].into(),
        [0.0, 2.0, 0.0].into(),
        [0.4, 0.4, 0.0].into(),
    ];
    let mesh = TriMesh::new(positions, vec![0, 1, 2, 1, 2, 3]);
    assert!(
        self_intersections(&mesh).is_empty(),
        "triangles sharing edge (1,2) are adjacent, however their far vertices lie"
    );
    assert_eq!(
        self_intersections(&mesh),
        self_intersections_brute_force(&mesh),
        "both paths must apply the same adjacency rule"
    );
}

/// Coplanar neighbours that only touch are no longer reported.
///
/// Two triangles tiling a square share an edge and lie in one plane. The
/// earlier conservative rule reported every coplanar pair, so any split
/// face's siblings looked self-intersecting. They share boundary, not area.
#[test]
fn coplanar_triangles_sharing_an_edge_do_not_overlap() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [1.0, 1.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
        ],
        vec![0, 1, 2, 0, 2, 3],
    );
    assert!(
        self_intersections_brute_force(&mesh).is_empty(),
        "coplanar triangles meeting along a shared edge only touch"
    );
}

/// Disjoint coplanar triangles are not reported.
///
/// Same plane, no shared vertex, no shared area. This is the case that made
/// every pair of sibling fragments in an exact boolean result look like a
/// defect.
#[test]
fn disjoint_coplanar_triangles_do_not_overlap() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
            [5.0, 5.0, 0.0].into(),
            [6.0, 5.0, 0.0].into(),
            [5.0, 6.0, 0.0].into(),
        ],
        vec![0, 1, 2, 3, 4, 5],
    );
    assert!(
        self_intersections_brute_force(&mesh).is_empty(),
        "coplanar triangles far apart share no area"
    );
}

/// Genuinely overlapping coplanar triangles ARE still reported.
///
/// The tightening must not turn into blanket silence: two triangles in one
/// plane whose interiors intersect are a real defect.
#[test]
fn overlapping_coplanar_triangles_are_reported() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [4.0, 0.0, 0.0].into(),
            [0.0, 4.0, 0.0].into(),
            [1.0, 1.0, 0.0].into(),
            [5.0, 1.0, 0.0].into(),
            [1.0, 5.0, 0.0].into(),
        ],
        vec![0, 1, 2, 3, 4, 5],
    );
    assert_eq!(
        self_intersections_brute_force(&mesh).len(),
        1,
        "coplanar triangles with shared interior area must still be found"
    );
}

/// One coplanar triangle strictly inside another is reported.
///
/// No edges cross here, so edge-crossing alone would miss it. Containment is
/// the second half of the test for a reason.
#[test]
fn a_coplanar_triangle_contained_in_another_is_reported() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [10.0, 0.0, 0.0].into(),
            [0.0, 10.0, 0.0].into(),
            [1.0, 1.0, 0.0].into(),
            [2.0, 1.0, 0.0].into(),
            [1.0, 2.0, 0.0].into(),
        ],
        vec![0, 1, 2, 3, 4, 5],
    );
    assert_eq!(
        self_intersections_brute_force(&mesh).len(),
        1,
        "a coplanar triangle inside another shares its whole area"
    );
}

/// Coplanar triangles meeting at a single vertex only touch.
#[test]
fn coplanar_triangles_sharing_one_vertex_do_not_overlap() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
            [-1.0, 0.0, 0.0].into(),
            [0.0, -1.0, 0.0].into(),
        ],
        vec![0, 1, 2, 0, 3, 4],
    );
    assert!(
        self_intersections_brute_force(&mesh).is_empty(),
        "a shared corner is boundary contact, not overlap"
    );
}

/// A coplanar vertex lying exactly ON another triangle's edge is contact.
///
/// This is the T-junction an exact boolean produces whenever one face is
/// split and its neighbour is not. The apex of the second triangle sits
/// exactly on the first's hypotenuse, so the two share boundary and no area.
/// Treating an on-edge point as interior would report every such junction.
#[test]
fn a_coplanar_vertex_on_an_edge_is_contact_not_overlap() {
    let mesh = TriMesh::new(
        vec![
            [0.0, 0.0, 0.0].into(),
            [4.0, 0.0, 0.0].into(),
            [0.0, 4.0, 0.0].into(),
            // Apex sits exactly on the hypotenuse x + y = 4, at (2, 2).
            [2.0, 2.0, 0.0].into(),
            [4.0, 4.0, 0.0].into(),
            [1.0, 5.0, 0.0].into(),
        ],
        vec![0, 1, 2, 3, 4, 5],
    );
    assert!(
        self_intersections_brute_force(&mesh).is_empty(),
        "a vertex resting on an edge touches without sharing area"
    );
}
