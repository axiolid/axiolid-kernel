//! Smoothing: the boundary is data, not an afterthought.

use axiolid_core::Point3;
use axiolid_mesh::TriMesh;
use axiolid_refine::smooth::{smooth, SmoothOptions};
use axiolid_refine::RefineError;

/// A flat grid with an open border: 4 triangles around one centre vertex.
fn open_patch() -> TriMesh {
    let positions = vec![
        Point3::new(-1.0, -1.0, 0.0),
        Point3::new(1.0, -1.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(-1.0, 1.0, 0.0),
        // Centre, deliberately pushed off the plane so smoothing has work.
        Point3::new(0.2, -0.1, 0.6),
    ];
    let indices = vec![0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4];
    TriMesh::new(positions, indices)
}

/// The headline guarantee: a fixed boundary is bit-identical, not just close.
#[test]
fn a_fixed_boundary_is_left_bit_identical() {
    let mesh = open_patch();
    let (out, report) = smooth(
        &mesh,
        SmoothOptions {
            factor: 1.0,
            passes: 8,
            fix_boundary: true,
        },
    )
    .expect("smooths");

    assert_eq!(
        report.boundary_vertices, 4,
        "the four corners are the border"
    );
    for corner in 0..4 {
        assert_eq!(
            out.positions[corner], mesh.positions[corner],
            "boundary vertex {corner} must be bit-identical after smoothing"
        );
    }
    assert_ne!(
        out.positions[4], mesh.positions[4],
        "the interior vertex must actually move, or the test proves nothing"
    );
}

/// Releasing the boundary must visibly change the result.
///
/// Without this, `fix_boundary` could be ignored entirely and the test above
/// would still pass.
#[test]
fn releasing_the_boundary_moves_it() {
    let mesh = open_patch();
    let (fixed, _) = smooth(&mesh, SmoothOptions::default()).expect("smooths");
    let (free, _) = smooth(
        &mesh,
        SmoothOptions {
            fix_boundary: false,
            ..SmoothOptions::default()
        },
    )
    .expect("smooths");

    assert_eq!(fixed.positions[0], mesh.positions[0]);
    assert_ne!(
        free.positions[0], mesh.positions[0],
        "with the boundary released, a border vertex must move"
    );
}

/// Smoothing pulls an interior vertex toward its neighbours' average.
#[test]
fn smoothing_moves_an_interior_vertex_toward_its_neighbours() {
    let mesh = open_patch();
    let before = mesh.positions[4];
    let (out, report) = smooth(
        &mesh,
        SmoothOptions {
            factor: 1.0,
            passes: 1,
            fix_boundary: true,
        },
    )
    .expect("smooths");

    // The four corners average to the origin, so one full-factor pass lands
    // the centre vertex exactly there.
    let after = out.positions[4];
    assert!(
        after.length() < 1e-12,
        "one full pass must land the centre on its neighbours' average, got {after:?}"
    );
    assert!(
        after.z.abs() < before.z.abs(),
        "the off-plane bulge must flatten"
    );
    assert_eq!(report.vertices_moved, 1);
    assert!(report.max_movement > 0.0, "movement must be reported");
}

#[test]
fn connectivity_is_preserved() {
    let mesh = open_patch();
    let (out, _) = smooth(&mesh, SmoothOptions::default()).expect("smooths");
    assert_eq!(
        out.indices, mesh.indices,
        "smoothing moves vertices; it must not retopologise"
    );
    assert_eq!(out.positions.len(), mesh.positions.len());
}

#[test]
fn an_invalid_relaxation_factor_is_refused() {
    let mesh = open_patch();
    for bad in [0.0, -0.5, 1.5, f64::NAN] {
        assert!(
            matches!(
                smooth(
                    &mesh,
                    SmoothOptions {
                        factor: bad,
                        ..SmoothOptions::default()
                    }
                ),
                Err(RefineError::InvalidTarget(_))
            ),
            "factor {bad} must be refused"
        );
    }
}
