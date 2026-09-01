use axiolid_core::{Point3, Tolerance};
use axiolid_measure::{WindingError, WindingMesh};
use axiolid_mesh::TriMesh;

fn point(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn tetrahedron() -> TriMesh {
    TriMesh::new(
        vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(0.0, 0.0, 1.0),
        ],
        vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    )
}

#[test]
fn closed_oriented_tetrahedron_has_unit_magnitude_inside_and_zero_outside() {
    let tetrahedron = tetrahedron();
    let mesh = WindingMesh::prepare(&tetrahedron, Tolerance::ZERO).expect("valid tetrahedron");

    let inside = mesh
        .winding_number(point(0.1, 0.1, 0.1))
        .expect("finite point");
    assert!((inside.value.abs() - 1.0).abs() < 1e-12, "{inside:?}");
    assert_eq!(inside.skipped_singular_triangles, 0);

    let outside = mesh
        .winding_number(point(2.0, 2.0, 2.0))
        .expect("finite point");
    assert!(outside.value.abs() < 1e-12, "{outside:?}");
    assert_eq!(outside.skipped_singular_triangles, 0);
}

#[test]
fn point_at_mesh_vertex_reports_skipped_singular_faces() {
    let tetrahedron = tetrahedron();
    let mesh = WindingMesh::prepare(&tetrahedron, Tolerance::ZERO).expect("valid tetrahedron");

    let result = mesh
        .winding_number(point(0.0, 0.0, 0.0))
        .expect("finite point");

    assert_eq!(result.skipped_singular_triangles, 3);
}

#[test]
fn invalid_mesh_is_rejected_with_structural_evidence() {
    let invalid = TriMesh::new(vec![point(0.0, 0.0, 0.0)], vec![0, 1, 2]);

    assert!(matches!(
        WindingMesh::prepare(&invalid, Tolerance::ZERO),
        Err(WindingError::MeshNotWindingUsable(_))
    ));
}

#[test]
fn non_finite_query_point_is_rejected() {
    let tetrahedron = tetrahedron();
    let mesh = WindingMesh::prepare(&tetrahedron, Tolerance::ZERO).expect("valid tetrahedron");

    assert_eq!(
        mesh.winding_number(point(f64::NAN, 0.0, 0.0)),
        Err(WindingError::NonFinitePoint)
    );
}
