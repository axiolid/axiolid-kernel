//! The general exact boolean agrees with the mesh oracle (#77).
//!
//! Lives here rather than in `axiolid-construct` because it drives a
//! PROVIDER (`boolmesh`), and an algorithms crate must not depend on a
//! provider even for tests -- the architecture gate rejects that inversion.
//! `axiolid-compile` legitimately depends on both layers.

use axiolid_construct::polyhedron::{boolean_polyhedra_exact, BooleanOp, Polyhedron};
use axiolid_contracts::ExecutionOptions;
use axiolid_core::{BooleanOperator, Point3, Tolerance};
use axiolid_measure::volume_properties;
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_boolean_contract::MeshBoolean;

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("tolerance")
}

fn box_solid(min: [f64; 3], max: [f64; 3]) -> Polyhedron {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    Polyhedron::new(vec![
        vec![p(x0, y0, z0), p(x0, y1, z0), p(x1, y1, z0), p(x1, y0, z0)],
        vec![p(x0, y0, z1), p(x1, y0, z1), p(x1, y1, z1), p(x0, y1, z1)],
        vec![p(x0, y0, z0), p(x1, y0, z0), p(x1, y0, z1), p(x0, y0, z1)],
        vec![p(x0, y1, z0), p(x0, y1, z1), p(x1, y1, z1), p(x1, y1, z0)],
        vec![p(x0, y0, z0), p(x0, y0, z1), p(x0, y1, z1), p(x0, y1, z0)],
        vec![p(x1, y0, z0), p(x1, y1, z0), p(x1, y1, z1), p(x1, y0, z1)],
    ])
    .expect("box")
}

fn to_mesh(solid: &Polyhedron) -> TriMesh {
    let mut positions: Vec<Point3> = Vec::new();
    let mut indices = Vec::new();
    let mut lookup: std::collections::HashMap<[u64; 3], u32> = std::collections::HashMap::new();
    for face in solid.faces() {
        let ring: Vec<u32> = face
            .iter()
            .map(|&p| {
                let key = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
                *lookup.entry(key).or_insert_with(|| {
                    positions.push(p);
                    (positions.len() - 1) as u32
                })
            })
            .collect();
        for i in 1..ring.len() - 1 {
            indices.extend([ring[0], ring[i], ring[i + 1]]);
        }
    }
    TriMesh::new(positions, indices)
}

fn volume(solid: &Polyhedron) -> f64 {
    volume_properties(&to_mesh(solid), tol())
        .expect("closed solid")
        .signed_volume
}
/// Differential against the mesh boolean, which shares no code with this.
///
/// The exact path splits polygons against input planes; `boolmesh` runs a
/// general mesh boolean in f64. Agreement on volume is evidence neither is
/// wrong in a way the other is not.
#[test]
fn the_exact_and_mesh_paths_agree_on_volume() {
    let a = box_solid([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_solid([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);

    for (op, mesh_op) in [
        (BooleanOp::Union, BooleanOperator::Union),
        (BooleanOp::Intersection, BooleanOperator::Intersection),
        (BooleanOp::Difference, BooleanOperator::Difference),
    ] {
        let exact = volume(&boolean_polyhedra_exact(&a, &b, op).expect("exact"));

        let outcome = BoolmeshBoolean::new()
            .boolean(
                &to_mesh(&a),
                &to_mesh(&b),
                mesh_op,
                &ExecutionOptions::new(tol()),
            )
            .expect("mesh boolean");
        let approximate = volume_properties(&outcome.mesh, tol())
            .expect("mesh result is closed")
            .signed_volume;

        assert!(
            (exact - approximate).abs() < 1e-9,
            "{op:?}: exact {exact} vs mesh {approximate}"
        );
    }
}
