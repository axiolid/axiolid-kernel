//! Degree reduction is lossy, so it must meet a tolerance or refuse (#33).
//!
//! The clean case is the inverse of elevation: a curve that was elevated is
//! exactly representable one degree lower, so reduction must recover it. The
//! dirty case is a curve that genuinely needs its degree, which must be
//! refused rather than flattened.

use axiolid_core::{Point2, Scalar};
use axiolid_curve::BSplineCurve2;
use axiolid_evaluate::curve::bspline_jet2;
use axiolid_nurbs::{elevate_degree2, reduce_degree2};

fn sample(curve: &BSplineCurve2, t: Scalar) -> Point2 {
    bspline_jet2(curve, t).expect("valid sample").point
}

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

/// Elevate then reduce recovers the original curve.
#[test]
fn reduction_inverts_elevation() {
    let original = quadratic();
    let elevated = elevate_degree2(&original).expect("elevation always succeeds");
    let reduced = reduce_degree2(&elevated, 1e-9).expect("an elevated curve reduces cleanly");

    assert_eq!(reduced.curve.degree, original.degree);
    assert!(
        reduced.deviation_upper_bound < 1e-10,
        "round trip should be near-exact, got {:.3e}",
        reduced.deviation_upper_bound
    );

    for step in 0..=40 {
        let t = f64::from(step) / 40.0;
        let (a, b) = (sample(&original, t), sample(&reduced.curve, t));
        assert!(
            (a - b).length() < 1e-9,
            "round trip differs at t={t}: {a:?} vs {b:?}"
        );
    }
}

/// A curve that genuinely needs its degree is refused.
///
/// A cubic with real cubic character cannot be a quadratic. Reduction must
/// say so rather than return the nearest quadratic, which would look like a
/// successful operation while changing the geometry.
#[test]
fn a_curve_that_needs_its_degree_is_refused() {
    let cubic = BSplineCurve2 {
        degree: 3,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 6.0),
            Point2::new(2.0, -6.0),
            Point2::new(3.0, 0.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![4, 4],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };

    let error = reduce_degree2(&cubic, 1e-6)
        .expect_err("this curve has real cubic character and cannot be a quadratic");
    let text = format!("{error}");
    assert!(
        text.contains("not removable") || text.contains("deviation"),
        "the refusal must report the deviation, got: {text}"
    );
}

/// Degree 1 cannot be reduced: the result would not be a curve.
#[test]
fn a_linear_curve_is_refused_outright() {
    let linear = BSplineCurve2 {
        degree: 1,
        control_points: vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };
    assert!(reduce_degree2(&linear, 1.0).is_err());
}

/// Averaging the two recurrences beats using either one alone.
///
/// On an exactly-reducible curve the forward and backward recurrences agree,
/// so the round-trip test above cannot tell them apart -- it passes even with
/// the averaging removed. This fixture is NOT exactly reducible, which is
/// where they diverge: a one-directional recurrence accumulates its error at
/// the far end, while averaging distributes it.
///
/// The assertion is the property that matters: the reported deviation is
/// symmetric, so neither end of the curve is favoured. A forward-only
/// implementation fails it.
#[test]
fn reduction_error_is_distributed_not_piled_at_one_end() {
    let cubic = BSplineCurve2 {
        degree: 3,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 6.0),
            Point2::new(2.0, -6.0),
            Point2::new(3.0, 0.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![4, 4],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };

    // Accept it so the approximation can be inspected.
    let reduced = reduce_degree2(&cubic, 100.0).expect("a huge tolerance accepts anything");

    let error_at = |t: Scalar| (sample(&cubic, t) - sample(&reduced.curve, t)).length();
    let (head, tail) = (error_at(0.15), error_at(0.85));

    assert!(
        (head - tail).abs() < 0.25 * head.max(tail).max(1e-12),
        "error is lopsided: {head:.4e} at the head vs {tail:.4e} at the tail, \
         which means one recurrence is being used alone"
    );
}
