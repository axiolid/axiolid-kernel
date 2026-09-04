//! Lofting runs a surface exactly through its sections (#33).
//!
//! The load-bearing case is the INTERIOR section. Using the sections directly
//! as control rows reproduces the first and last exactly while missing the
//! ones between, which looks correct in a two-section test and is wrong the
//! moment a third is added. So the fixtures use three sections and check the
//! middle one.

use axiolid_core::{Point3, Scalar};
use axiolid_curve::{BSplineCurve3, KnotSpec};
use axiolid_evaluate::surface::bspline_jet;
use axiolid_nurbs::loft_surface;

/// A straight two-point section at height `z`, offset in x by `bulge`.
fn section(z: Scalar, bulge: Scalar) -> BSplineCurve3 {
    BSplineCurve3 {
        degree: 1,
        control_points: vec![Point3::new(bulge, 0.0, z), Point3::new(bulge + 1.0, 2.0, z)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

/// The surface passes through the INTERIOR section, not just the ends.
///
/// The middle section is offset in x, so a loft that merely blends between
/// the outer two misses it by that offset.
#[test]
fn the_surface_passes_through_an_interior_section() {
    let sections = vec![section(0.0, 0.0), section(1.0, 3.0), section(2.0, 0.0)];
    let surface = loft_surface(&sections).expect("three compatible sections loft");

    // Sections are evenly spaced in the u parameter here, so the middle one
    // sits at u = 0.5.
    for step in 0..=8 {
        let v = f64::from(step) / 8.0;
        let point = bspline_jet(&surface, 0.5, v).expect("valid sample").point;
        // The middle section is the straight line from (3,0,1) to (4,2,1).
        let expected = Point3::new(3.0 + v, 2.0 * v, 1.0);
        assert!(
            (point - expected).length() < 1e-9,
            "interior section missed at v={v}: expected {expected:?}, got {point:?}"
        );
    }
}

/// The first and last sections are reproduced exactly.
#[test]
fn the_boundary_sections_are_reproduced() {
    let sections = vec![section(0.0, 0.0), section(1.0, 3.0), section(2.0, 0.0)];
    let surface = loft_surface(&sections).expect("three compatible sections loft");

    for (u, z) in [(0.0, 0.0), (1.0, 2.0)] {
        for step in 0..=4 {
            let v = f64::from(step) / 4.0;
            let point = bspline_jet(&surface, u, v).expect("valid sample").point;
            let expected = Point3::new(v, 2.0 * v, z);
            assert!(
                (point - expected).length() < 1e-9,
                "boundary section at u={u} missed at v={v}: got {point:?}"
            );
        }
    }
}

/// Mismatched section degrees are refused rather than silently reconciled.
///
/// Elevating one section to match another changes that curve's
/// representation. Doing it inside a loft would alter the caller's input in a
/// call that looks like pure construction, so the caller elevates explicitly.
#[test]
fn mismatched_section_degrees_are_refused() {
    let mut odd = section(1.0, 0.0);
    odd.degree = 2;
    odd.control_points.push(Point3::new(2.0, 4.0, 1.0));
    odd.multiplicities = vec![3, 3];

    let sections = vec![section(0.0, 0.0), odd];
    let error = loft_surface(&sections).expect_err("degrees differ, so this must refuse");
    let text = format!("{error}");
    assert!(
        text.contains("degree"),
        "the refusal must name the mismatch, got: {text}"
    );
}

/// One section is not a surface.
#[test]
fn a_single_section_is_refused() {
    assert!(loft_surface(&[section(0.0, 0.0)]).is_err());
}

/// Coincident sections make the spacing undefined.
#[test]
fn coincident_sections_are_refused() {
    let sections = vec![section(0.0, 0.0), section(0.0, 0.0)];
    assert!(loft_surface(&sections).is_err());
}
