//! A bounded half-space against its closed-form volume.
//!
//! The construction is a prism: the boundary profile swept along the plane
//! normal by a depth derived from the boundary's own extent. Its volume is
//! therefore area x depth, a constant this crate does not get to choose.

use axiolid_core::{Plane3, Point2, Point3, Scalar, Tolerance, Vec3};
use axiolid_generate::half_space::bounded_half_space;
use axiolid_generate::profile::Rings;
use axiolid_measure::volume_properties;
use axiolid_mesh::TriMesh;
use axiolid_primitive::ClipMargin;

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("tolerance")
}

fn volume(mesh: &TriMesh) -> Scalar {
    volume_properties(mesh, tol())
        .expect("a bounded half-space must be closed and two-manifold")
        .signed_volume
}

/// A square boundary of half-width `h`, wound counter-clockwise.
fn square(h: Scalar) -> Rings {
    Rings {
        outer: vec![
            Point2::new(-h, -h),
            Point2::new(h, -h),
            Point2::new(h, h),
            Point2::new(-h, h),
        ],
        holes: Vec::new(),
    }
}

fn plane_z() -> Plane3 {
    Plane3 {
        origin: Point3::ZERO,
        normal: Vec3::Z,
    }
}

#[test]
fn a_bounded_half_space_is_area_times_depth() {
    // Square of half-width 5 => area 100, extent 5, margin 2 => depth 10.
    let margin = ClipMargin::new(2.0).expect("margin");
    let mesh = bounded_half_space(&square(5.0), plane_z(), true, margin, tol()).expect("clip");
    let want = 100.0 * 10.0;
    let got = volume(&mesh);
    assert!(
        (got - want).abs() / want < 1e-9,
        "bounded half-space {got} vs {want}"
    );
}

#[test]
fn the_margin_scales_the_depth_proportionally() {
    // Doubling the margin must double the volume and nothing else: this is
    // what proves the depth is derived from the margin rather than from a
    // constant that happens to fit the first test.
    let a = bounded_half_space(
        &square(5.0),
        plane_z(),
        true,
        ClipMargin::new(2.0).expect("margin"),
        tol(),
    )
    .expect("clip");
    let b = bounded_half_space(
        &square(5.0),
        plane_z(),
        true,
        ClipMargin::new(4.0).expect("margin"),
        tol(),
    )
    .expect("clip");
    let ratio = volume(&b) / volume(&a);
    assert!((ratio - 2.0).abs() < 1e-9, "margin ratio {ratio} vs 2");
}

#[test]
fn the_construction_is_unit_independent() {
    // The same boundary scaled by 1000 must give exactly 1000^3 the volume.
    // Sizing the slab from the boundary's own extent is what guarantees
    // this; an absolute constant would break it.
    let margin = ClipMargin::new(2.0).expect("margin");
    let small = bounded_half_space(&square(5.0), plane_z(), true, margin, tol()).expect("clip");
    let large = bounded_half_space(&square(5000.0), plane_z(), true, margin, tol()).expect("clip");
    let ratio = volume(&large) / volume(&small);
    assert!(
        (ratio - 1e9).abs() / 1e9 < 1e-9,
        "unit scaling {ratio} vs 1e9"
    );
}

#[test]
fn agreement_selects_the_opposite_side() {
    // Both sides must be genuine solids of equal volume, and they must sit
    // on opposite sides of the plane. Equal volume alone would also hold if
    // agreement were ignored, so the z extent is what makes this a real test.
    let margin = ClipMargin::new(2.0).expect("margin");
    let up = bounded_half_space(&square(5.0), plane_z(), true, margin, tol()).expect("clip");
    let down = bounded_half_space(&square(5.0), plane_z(), false, margin, tol()).expect("clip");
    assert!((volume(&up) - volume(&down)).abs() < 1e-9, "equal volume");

    let max_z = |m: &TriMesh| m.positions.iter().fold(Scalar::MIN, |a, p| a.max(p.z));
    let min_z = |m: &TriMesh| m.positions.iter().fold(Scalar::MAX, |a, p| a.min(p.z));
    assert!(
        max_z(&up) > 0.0 && min_z(&up) >= -1e-12,
        "normal side is +z"
    );
    assert!(
        min_z(&down) < 0.0 && max_z(&down) <= 1e-12,
        "opposite side is -z"
    );
}

#[test]
fn a_degenerate_boundary_is_refused() {
    let margin = ClipMargin::new(2.0).expect("margin");
    // Zero extent cannot size a slab. The ring has three distinct points
    // so it passes the vertex-count check and reaches the extent guard,
    // which is the condition actually under test here.
    let flat = Rings {
        outer: vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0),
        ],
        holes: Vec::new(),
    };
    assert!(bounded_half_space(&flat, plane_z(), true, margin, tol()).is_err());
    assert!(bounded_half_space(&square(0.0), plane_z(), true, margin, tol()).is_err());
    // A zero normal has no side to select.
    let bad_plane = Plane3 {
        origin: Point3::ZERO,
        normal: Vec3::ZERO,
    };
    assert!(bounded_half_space(&square(5.0), bad_plane, true, margin, tol()).is_err());
    // Two points do not bound a region.
    let sliver = Rings {
        outer: vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        holes: Vec::new(),
    };
    assert!(bounded_half_space(&sliver, plane_z(), true, margin, tol()).is_err());
}
