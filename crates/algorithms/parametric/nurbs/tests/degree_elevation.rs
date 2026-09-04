//! Degree elevation is exact (#33).
//!
//! Elevation raises the degree without moving the curve. The test that proves
//! it is sampling both curves at the same parameters and requiring agreement
//! to near machine precision -- comparing control points would only prove the
//! arithmetic ran, not that the geometry is unchanged.

use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{BSplineCurve2, BSplineCurve3};
use axiolid_evaluate::curve::{bspline_jet2, bspline_jet3};
use axiolid_nurbs::{elevate_degree2, elevate_degree3};

/// A clamped quadratic with two spans, so elevation must handle an internal knot.
fn quadratic() -> BSplineCurve2 {
    BSplineCurve2 {
        degree: 2,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 2.0),
            Point2::new(3.0, -1.0),
            Point2::new(4.0, 1.0),
        ],
        knots: vec![0.0, 0.5, 1.0],
        multiplicities: vec![3, 1, 3],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

/// A rational quadratic: a quarter circle, where weights matter.
///
/// A polynomial-only elevation would move this curve, so it catches an
/// implementation that blends positions while ignoring weights.
fn rational_arc() -> BSplineCurve2 {
    let w = core::f64::consts::FRAC_1_SQRT_2;
    BSplineCurve2 {
        degree: 2,
        control_points: vec![
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![3, 3],
        weights: Some(vec![1.0, w, 1.0]),
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

fn sample2(curve: &BSplineCurve2, t: Scalar) -> Point2 {
    bspline_jet2(curve, t).expect("valid curve sample").point
}

#[test]
fn elevating_a_polynomial_curve_does_not_move_it() {
    let original = quadratic();
    let elevated = elevate_degree2(&original).expect("elevation is always valid here");

    assert_eq!(elevated.degree, 3, "degree must increase by exactly one");

    for step in 0..=40 {
        let t = f64::from(step) / 40.0;
        let (a, b) = (sample2(&original, t), sample2(&elevated, t));
        assert!(
            (a - b).length() < 1e-12,
            "curves diverge at t={t}: {a:?} vs {b:?}"
        );
    }
}

#[test]
fn elevating_a_rational_curve_preserves_its_weights_effect() {
    let original = rational_arc();
    let elevated = elevate_degree2(&original).expect("elevation is always valid here");

    assert_eq!(elevated.degree, 3);
    assert!(
        elevated.weights.is_some(),
        "a rational curve must stay rational"
    );

    for step in 0..=40 {
        let t = f64::from(step) / 40.0;
        let (a, b) = (sample2(&original, t), sample2(&elevated, t));
        assert!(
            (a - b).length() < 1e-12,
            "rational curves diverge at t={t}: {a:?} vs {b:?}"
        );
    }
}

/// The spatial path elevates exactly too.
///
/// `elevate_degree3` shares the generic core with the planar path, but the
/// coordinate extraction differs, so a 3D fixture is what proves the z axis is
/// carried rather than dropped.
#[test]
fn elevating_a_spatial_curve_does_not_move_it() {
    let original = BSplineCurve3 {
        degree: 2,
        control_points: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 1.0),
            Point3::new(3.0, -1.0, 2.0),
            Point3::new(4.0, 1.0, -1.0),
        ],
        knots: vec![0.0, 0.5, 1.0],
        multiplicities: vec![3, 1, 3],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };
    let elevated = elevate_degree3(&original).expect("elevation is always valid here");

    assert_eq!(elevated.degree, 3);
    for step in 0..=40 {
        let t = f64::from(step) / 40.0;
        let a = bspline_jet3(&original, t).expect("valid sample").point;
        let b = bspline_jet3(&elevated, t).expect("valid sample").point;
        assert!(
            (a - b).length() < 1e-12,
            "spatial curves diverge at t={t}: {a:?} vs {b:?}"
        );
    }
}
