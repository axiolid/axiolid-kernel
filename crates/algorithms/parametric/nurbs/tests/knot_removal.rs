//! Knot removal is lossy, so it must meet a tolerance or refuse (#33).
//!
//! Two properties, and the second is the one that matters:
//!
//! - a knot that was INSERTED can be removed cleanly, recovering the original;
//! - a knot that carries real shape is REFUSED, not silently approximated.
//!
//! An implementation that always returns something passes the first test and
//! fails the second, which is exactly the silent-approximation failure the
//! issue calls out.

use axiolid_core::{Point2, Scalar};
use axiolid_curve::BSplineCurve2;
use axiolid_evaluate::curve::bspline_jet2;
use axiolid_nurbs::{insert_knot2, remove_knot2};

fn sample(curve: &BSplineCurve2, t: Scalar) -> Point2 {
    bspline_jet2(curve, t).expect("valid sample").point
}

/// A clamped cubic with a single span: no internal knots to start with.
fn cubic() -> BSplineCurve2 {
    BSplineCurve2 {
        degree: 3,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 3.0),
            Point2::new(3.0, -2.0),
            Point2::new(4.0, 1.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![4, 4],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

/// An inserted knot is removable, and removing it recovers the curve.
///
/// Insertion is exact and shape-preserving, so the knot it adds carries no
/// shape. Removal must therefore succeed with essentially zero deviation.
#[test]
fn a_knot_that_was_inserted_can_be_removed_cleanly() {
    let original = cubic();
    let refined = insert_knot2(&original, 0.5).expect("0.5 is interior");
    assert_eq!(
        refined.control_points.len(),
        original.control_points.len() + 1,
        "insertion must add exactly one control point"
    );

    let removed = remove_knot2(&refined, 0.5, 1e-9).expect("an inserted knot must be removable");

    assert!(
        removed.deviation_upper_bound < 1e-12,
        "removing an inserted knot should be near-exact, got {:.3e}",
        removed.deviation_upper_bound
    );
    assert_eq!(
        removed.curve.control_points.len(),
        original.control_points.len(),
        "removal must drop exactly one control point"
    );

    for step in 0..=40 {
        let t = f64::from(step) / 40.0;
        let (a, b) = (sample(&original, t), sample(&removed.curve, t));
        assert!(
            (a - b).length() < 1e-10,
            "recovered curve differs at t={t}: {a:?} vs {b:?}"
        );
    }
}

/// A knot carrying real shape is REFUSED, not silently approximated.
///
/// This is the property the issue is actually about. The curve below has a
/// sharp direction change at its internal knot, so that knot is load-bearing:
/// removing it must change the shape. The operation must say so rather than
/// return a smoothed curve that looks reasonable.
#[test]
fn a_shape_carrying_knot_is_refused() {
    // Two spans meeting at 0.5 with control points that force a kink.
    let kinked = BSplineCurve2 {
        degree: 2,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 4.0),
            Point2::new(2.0, -4.0),
            Point2::new(3.0, 4.0),
            Point2::new(4.0, 0.0),
        ],
        knots: vec![0.0, 0.4, 0.7, 1.0],
        multiplicities: vec![3, 1, 1, 3],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };

    let error = remove_knot2(&kinked, 0.4, 1e-6)
        .expect_err("a knot carrying this much shape cannot be removed within 1e-6");

    let text = format!("{error}");
    assert!(
        text.contains("not removable"),
        "the refusal must say the knot is not removable, got: {text}"
    );
}

/// The same knot is accepted when the caller's tolerance is loose enough.
///
/// The refusal is a function of the caller's budget, not a fixed opinion. A
/// caller who genuinely accepts a coarse approximation gets one -- and gets
/// told exactly how coarse it was.
#[test]
fn a_loose_tolerance_accepts_and_reports_the_real_deviation() {
    let kinked = BSplineCurve2 {
        degree: 2,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 4.0),
            Point2::new(2.0, -4.0),
            Point2::new(3.0, 4.0),
            Point2::new(4.0, 0.0),
        ],
        knots: vec![0.0, 0.4, 0.7, 1.0],
        multiplicities: vec![3, 1, 1, 3],
        weights: None,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };

    let removed = remove_knot2(&kinked, 0.4, 100.0).expect("a huge tolerance accepts anything");

    assert!(
        removed.deviation_upper_bound > 1e-6,
        "the reported deviation must be the real one, got {:.3e}",
        removed.deviation_upper_bound
    );

    // And the reported number must match what the curves actually do.
    let mut worst: Scalar = 0.0;
    for step in 0..=128 {
        let t = f64::from(step) / 128.0;
        let a = sample(&kinked, t);
        let b = sample(&removed.curve, t);
        worst = worst.max((a - b).length());
    }
    assert!(
        (worst - removed.deviation_upper_bound).abs() < 1e-9,
        "reported {:.6e} but measured {:.6e}",
        removed.deviation_upper_bound,
        worst
    );
}
