use axiolid_core::{Point3, Tolerance};
use axiolid_measure::{surface_properties, volume_properties};
use axiolid_mesh::TriMesh;

fn tetra() -> TriMesh {
    TriMesh::new(
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ],
        vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3],
    )
}

#[test]
fn tetra_surface_and_volume_properties_are_raw_and_deterministic() {
    let mesh = tetra();
    let surface = surface_properties(&mesh, Tolerance::ZERO).unwrap();
    assert!((surface.area - (1.5 + 3.0_f64.sqrt() / 2.0)).abs() < 1e-12);
    let expected_surface_coordinate = (2.0 + 3.0_f64.sqrt()) / (3.0 * (3.0 + 3.0_f64.sqrt()));
    assert!((surface.centroid.x - expected_surface_coordinate).abs() < 1e-12);
    assert!((surface.centroid.y - expected_surface_coordinate).abs() < 1e-12);
    assert!((surface.centroid.z - expected_surface_coordinate).abs() < 1e-12);
    let volume = volume_properties(&mesh, Tolerance::ZERO).unwrap();
    assert!((volume.signed_volume - 1.0 / 6.0).abs() < 1e-12);
    assert_eq!(volume.centroid, Point3::splat(0.25));
}

#[test]
fn volume_rejects_open_mesh() {
    let mut mesh = tetra();
    mesh.indices.truncate(9);
    assert!(volume_properties(&mesh, Tolerance::ZERO).is_err());
}

#[test]
fn volume_rejects_closed_mesh_with_inconsistent_winding() {
    let mut mesh = tetra();
    mesh.indices[..3].reverse();
    assert!(volume_properties(&mesh, Tolerance::ZERO).is_err());
}
