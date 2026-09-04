//! Diagnosis is produced from real evidence, not vocabulary (#73).
//!
//! `Diagnosis::blocks_boolean` existed before anything could produce a
//! `Diagnosis`, so it was a function over data that never arrived. These
//! tests pin that it now answers from measured defects.

use axiolid_core::Tolerance;
use axiolid_heal::{diagnose, DefectKind};
use axiolid_mesh::TriMesh;

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("valid tolerance")
}

fn unit_cube() -> TriMesh {
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
    TriMesh::new(positions, indices)
}

#[test]
fn a_closed_cube_is_diagnosed_clean() {
    let diagnosis = diagnose(&unit_cube(), tol());
    assert!(
        diagnosis.is_clean(),
        "a valid solid must produce no defects: {:?}",
        diagnosis.defects
    );
    assert!(!diagnosis.blocks_boolean());
}

#[test]
fn an_open_shell_is_reported_and_blocks_a_boolean() {
    // One face of the cube removed: the hole leaves boundary edges.
    let mut mesh = unit_cube();
    mesh.indices.truncate(mesh.indices.len() - 6);

    let diagnosis = diagnose(&mesh, tol());
    assert!(
        diagnosis
            .defects
            .iter()
            .any(|d| d.kind == DefectKind::OpenShell),
        "removing a face leaves boundary edges: {:?}",
        diagnosis.defects
    );
    assert!(
        diagnosis.blocks_boolean(),
        "an open shell has no well-defined inside, so a boolean cannot proceed"
    );
}

#[test]
fn a_self_intersecting_mesh_names_the_crossing_triangles() {
    // Two boxes overlapping by half their width, in one mesh: closed and
    // structurally valid, yet self-intersecting.
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

    let diagnosis = diagnose(&mesh, tol());
    let crossings: Vec<_> = diagnosis
        .defects
        .iter()
        .filter(|d| d.kind == DefectKind::SelfIntersection)
        .collect();
    assert!(
        !crossings.is_empty(),
        "overlapping boxes must be reported: {:?}",
        diagnosis.defects
    );
    assert!(
        crossings.iter().all(|d| d.element.is_some()),
        "a self-intersection must name WHICH triangle, not just that one exists"
    );
    assert!(
        diagnosis.blocks_boolean(),
        "self-intersection is the defect class that most reliably produces a \
         plausible-looking wrong boolean result"
    );
}
