//! Convex hull correctness, checked with exact predicates (#76).
//!
//! The oracles here are independent of the hull code: closed-form volume via
//! the v0.7 measurement provider, and containment re-decided with the same
//! certified `orient3d` the hull used — but applied to EVERY point against
//! EVERY face, which the incremental construction never does.

use axiolid_construct::hull::convex_hull;
use axiolid_core::{Point3, Tolerance};
use axiolid_guarantees::Sign;
use axiolid_measure::volume_properties;
use axiolid_mesh::{audit_mesh, TriMesh};
use axiolid_predicates::orient3d;

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("tolerance")
}

fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

/// The eight corners of the unit cube.
fn cube_corners() -> Vec<Point3> {
    vec![
        p(0.0, 0.0, 0.0),
        p(1.0, 0.0, 0.0),
        p(1.0, 1.0, 0.0),
        p(0.0, 1.0, 0.0),
        p(0.0, 0.0, 1.0),
        p(1.0, 0.0, 1.0),
        p(1.0, 1.0, 1.0),
        p(0.0, 1.0, 1.0),
    ]
}

/// Every input point lies inside or on every face, decided exactly.
///
/// This is the defining property of a hull, and it is checked against all
/// faces rather than the ones the incremental step happened to look at. No
/// tolerance appears: `orient3d` returns a certified sign.
fn assert_encloses(mesh: &TriMesh, points: &[Point3]) {
    let count = mesh.indices.len() / 3;
    for point in points {
        for t in 0..count {
            let a = mesh.positions[mesh.indices[t * 3] as usize];
            let b = mesh.positions[mesh.indices[t * 3 + 1] as usize];
            let c = mesh.positions[mesh.indices[t * 3 + 2] as usize];
            assert_ne!(
                orient3d(a, b, c, *point).sign(),
                Some(Sign::Negative),
                "point {point:?} lies outside face {t}"
            );
        }
    }
}

#[test]
fn the_hull_of_a_cubes_corners_is_that_cube() {
    let corners = cube_corners();
    let hull = convex_hull(&corners).expect("a cube is hullable");

    let health = audit_mesh(&hull, tol());
    assert!(
        health.is_closed_two_manifold(),
        "a hull must be a closed manifold: {health:?}"
    );

    let props = volume_properties(&hull, tol()).expect("closed hull measures");
    assert!(
        (props.signed_volume - 1.0).abs() < 1e-12,
        "unit cube hull volume: expected 1, got {}",
        props.signed_volume
    );
    assert_encloses(&hull, &corners);
}

#[test]
fn interior_points_do_not_change_the_hull() {
    let corners = cube_corners();
    let mut with_interior = corners.clone();
    // Points strictly inside, including one very near a face.
    with_interior.push(p(0.5, 0.5, 0.5));
    with_interior.push(p(0.25, 0.75, 0.5));
    with_interior.push(p(0.5, 0.5, 0.999));

    let plain = convex_hull(&corners).expect("hullable");
    let padded = convex_hull(&with_interior).expect("hullable");

    let a = volume_properties(&plain, tol()).expect("measures");
    let b = volume_properties(&padded, tol()).expect("measures");
    assert!(
        (a.signed_volume - b.signed_volume).abs() < 1e-12,
        "interior points changed the hull volume: {} vs {}",
        a.signed_volume,
        b.signed_volume
    );
    assert_encloses(&padded, &with_interior);
}

#[test]
fn points_sampled_inside_a_solid_reproduce_its_volume() {
    // A tetrahedron plus points strictly inside it. The hull must be the
    // tetrahedron, whose volume is known in closed form: |det| / 6.
    let mut points = vec![
        p(0.0, 0.0, 0.0),
        p(3.0, 0.0, 0.0),
        p(0.0, 4.0, 0.0),
        p(0.0, 0.0, 5.0),
    ];
    points.push(p(0.4, 0.4, 0.4));
    points.push(p(0.2, 1.0, 0.6));

    let hull = convex_hull(&points).expect("hullable");
    let props = volume_properties(&hull, tol()).expect("measures");
    let expected = 3.0 * 4.0 * 5.0 / 6.0;
    assert!(
        (props.signed_volume - expected).abs() < 1e-12,
        "tetrahedron volume: expected {expected}, got {}",
        props.signed_volume
    );
}

#[test]
fn a_coplanar_input_is_refused_as_coplanar() {
    // A square in the z = 0 plane, plus a point still in that plane. There
    // is no volume to enclose, and the caller can act on knowing it was
    // planar by falling back to the 2D hull.
    let flat = vec![
        p(0.0, 0.0, 0.0),
        p(1.0, 0.0, 0.0),
        p(1.0, 1.0, 0.0),
        p(0.0, 1.0, 0.0),
        p(0.5, 0.5, 0.0),
    ];
    let error = convex_hull(&flat).expect_err("a plane encloses no volume");
    let text = error.to_string();
    assert!(
        text.contains("coplanar"),
        "the refusal must name coplanarity, got: {text}"
    );
}

#[test]
fn a_collinear_input_is_refused_as_collinear() {
    let line: Vec<Point3> = (0..5).map(|i| p(f64::from(i), 0.0, 0.0)).collect();
    let error = convex_hull(&line).expect_err("a line encloses no volume");
    let text = error.to_string();
    assert!(
        text.contains("collinear"),
        "the refusal must name collinearity, got: {text}"
    );
}

#[test]
fn too_few_points_is_refused_by_count() {
    let three = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
    let error = convex_hull(&three).expect_err("3 points cannot bound a volume");
    assert!(error.to_string().contains("at least 4"));
}

#[test]
fn duplicate_points_do_not_produce_degenerate_faces() {
    let mut corners = cube_corners();
    // Repeat every corner: a naive hull would emit zero-area faces.
    corners.extend(cube_corners());
    let hull = convex_hull(&corners).expect("duplicates are absorbed");

    let health = audit_mesh(&hull, tol());
    assert_eq!(
        health.degenerate_triangles, 0,
        "duplicated input produced degenerate faces: {health:?}"
    );
    let props = volume_properties(&hull, tol()).expect("measures");
    assert!((props.signed_volume - 1.0).abs() < 1e-12);
}
