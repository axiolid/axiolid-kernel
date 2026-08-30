//! CSG primitives against closed-form volume.
//!
//! Each primitive has a known volume, so the mesh is checked against that
//! rather than against itself. A tessellated curved primitive is inscribed,
//! so it under-estimates and converges from below as the budget tightens --
//! asserting that direction catches an outward-bulging mesh too.

use axiolid_core::{Scalar, Tolerance};
use axiolid_measure::volume_properties;
use axiolid_mesh::TriMesh;
use axiolid_primitive::Primitive;
use axiolid_scalar::primitive::tessellate_primitive;

fn volume(mesh: &TriMesh, tol: Tolerance) -> Scalar {
    volume_properties(mesh, tol)
        .expect("a primitive must be a closed two-manifold solid")
        .signed_volume
}

/// Tolerance scaled to the chord budget: a curved primitive's facets are
/// legitimately thin, and judging them at MILLIMETRE calls correct geometry
/// degenerate.
fn tol_for(chord: Scalar) -> Tolerance {
    Tolerance::new((chord * 1e-3).max(1e-12), 1e-9).expect("tolerance")
}

#[test]
fn a_block_has_exactly_its_analytic_volume() {
    let p = Primitive::Block {
        x: 3.0,
        y: 2.0,
        z: 4.0,
    };
    let mesh = tessellate_primitive(&p, Tolerance::MILLIMETRE).expect("block");
    let v = volume(&mesh, Tolerance::MILLIMETRE);
    // Flat faces: no approximation, so this is exact.
    assert!((v - 24.0).abs() < 1e-9, "block volume {v}");
}

#[test]
fn a_pyramid_is_one_third_base_times_height() {
    let p = Primitive::Pyramid {
        x: 3.0,
        y: 3.0,
        height: 4.0,
    };
    let mesh = tessellate_primitive(&p, Tolerance::MILLIMETRE).expect("pyramid");
    let v = volume(&mesh, Tolerance::MILLIMETRE);
    assert!((v - 12.0).abs() < 1e-9, "pyramid volume {v}");
}

#[test]
fn a_cylinder_converges_on_pi_r_squared_h() {
    let chord = 1e-5;
    let p = Primitive::Cylinder {
        radius: 2.0,
        height: 5.0,
    };
    let mesh = tessellate_primitive(&p, Tolerance::new(chord, 1e-9).expect("t")).expect("cylinder");
    let v = volume(&mesh, tol_for(chord));
    let want = core::f64::consts::PI * 4.0 * 5.0;
    assert!(
        v < want,
        "an inscribed cylinder under-estimates: {v} vs {want}"
    );
    assert!((want - v) / want < 1e-5, "cylinder volume {v} vs {want}");
}

#[test]
fn a_cone_is_one_third_of_its_cylinder() {
    let chord = 1e-5;
    let p = Primitive::Cone {
        radius: 2.0,
        height: 5.0,
    };
    let mesh = tessellate_primitive(&p, Tolerance::new(chord, 1e-9).expect("t")).expect("cone");
    let v = volume(&mesh, tol_for(chord));
    let want = core::f64::consts::PI * 4.0 * 5.0 / 3.0;
    assert!((want - v) / want < 1e-5, "cone volume {v} vs {want}");
}

#[test]
fn a_sphere_converges_on_four_thirds_pi_r_cubed() {
    let chord = 1e-5;
    let p = Primitive::Sphere { radius: 2.0 };
    let mesh = tessellate_primitive(&p, Tolerance::new(chord, 1e-9).expect("t")).expect("sphere");
    let v = volume(&mesh, tol_for(chord));
    let want = 4.0 / 3.0 * core::f64::consts::PI * 8.0;
    assert!((want - v) / want < 1e-4, "sphere volume {v} vs {want}");
}

#[test]
fn a_tighter_budget_converges() {
    let want = core::f64::consts::PI * 4.0 * 5.0;
    let mut previous = Scalar::INFINITY;
    for chord in [1e-2, 1e-3, 1e-4, 1e-5] {
        let p = Primitive::Cylinder {
            radius: 2.0,
            height: 5.0,
        };
        let mesh =
            tessellate_primitive(&p, Tolerance::new(chord, 1e-9).expect("t")).expect("cylinder");
        let error = (want - volume(&mesh, tol_for(chord))).abs();
        assert!(error < previous, "error must shrink: {error} vs {previous}");
        previous = error;
    }
}

#[test]
fn a_non_positive_extent_is_refused() {
    let p = Primitive::Sphere { radius: 0.0 };
    assert!(tessellate_primitive(&p, Tolerance::MILLIMETRE).is_err());
}
