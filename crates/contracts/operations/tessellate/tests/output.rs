//! Tessellation results preserve the policy evidence needed by downstream users.

use axiolid_core::{Tolerance, Vec3};
use axiolid_mesh::TriMesh;
use axiolid_tessellation_contract::TessellatedMesh;

#[test]
fn produced_mesh_carries_the_tolerance_that_created_it() {
    let tolerance = Tolerance::new(0.0025, 1.0e-6).expect("valid tolerance");
    let mesh = TriMesh::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![0, 1, 2]);

    let output = TessellatedMesh::new(mesh.clone(), tolerance);

    assert_eq!(output.tolerance(), tolerance);
    assert_eq!(output.mesh(), &mesh);
    assert_eq!(output.into_mesh(), mesh);
}
