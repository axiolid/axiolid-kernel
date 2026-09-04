//! Interpolation passes THROUGH its points (#33).
//!
//! The contract is interpolation, not approximation: a caller handing over
//! survey points or a section outline needs those points on the curve. So the
//! test evaluates the result at each computed parameter and requires the input
//! point back to near machine precision -- a fit that merely comes close would
//! pass a tolerance-based check while breaking the contract.

use axiolid_core::{Point3, Scalar};
use axiolid_curve::BSplineCurve3;
use axiolid_evaluate::curve::bspline_jet3;
use axiolid_nurbs::interpolate_curve3;

fn sample(curve: &BSplineCurve3, t: Scalar) -> Point3 {
    bspline_jet3(curve, t).expect("valid sample").point
}

/// Recover the chord-length parameters the implementation assigns.
///
/// Computed independently here rather than returned by the API, so the test
/// checks the interpolation property rather than trusting the implementation's
/// own bookkeeping.
fn chord_parameters(points: &[Point3]) -> Vec<Scalar> {
    let mut distances = vec![0.0];
    let mut total = 0.0;
    for pair in points.windows(2) {
        total += (pair[1] - pair[0]).length();
        distances.push(total);
    }
    distances.into_iter().map(|d| d / total).collect()
}

#[test]
fn the_curve_passes_through_every_input_point() {
    // Unevenly spaced deliberately: uniform parameterisation on uneven points
    // overshoots, so this also exercises the chord-length choice.
    let points = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, 0.5),
        Point3::new(4.0, 1.0, -1.0),
        Point3::new(4.5, -2.0, 0.0),
        Point3::new(8.0, 0.0, 2.0),
    ];

    let curve = interpolate_curve3(&points).expect("five distinct points interpolate");
    let parameters = chord_parameters(&points);

    for (index, (&t, expected)) in parameters.iter().zip(&points).enumerate() {
        let actual = sample(&curve, t);
        assert!(
            (actual - *expected).length() < 1e-9,
            "point {index} is not on the curve: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn endpoints_are_interpolated_exactly() {
    let points = vec![
        Point3::new(-3.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 4.0, 1.0),
        Point3::new(7.0, 1.0, -2.0),
    ];
    let curve = interpolate_curve3(&points).expect("four distinct points interpolate");

    let first = sample(&curve, 0.0);
    let last = sample(&curve, 1.0);
    assert!(
        (first - points[0]).length() < 1e-12,
        "start moved: {first:?}"
    );
    assert!(
        (last - points[points.len() - 1]).length() < 1e-12,
        "end moved: {last:?}"
    );
}

/// Coincident consecutive points make chord length undefined, so refuse.
///
/// Silently merging them would change the caller's data; parameterising them
/// as a zero-length span divides by zero.
#[test]
fn coincident_points_are_refused() {
    let points = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
        Point3::new(2.0, 0.0, 0.0),
    ];
    assert!(interpolate_curve3(&points).is_err());
}

/// One point is not a curve.
#[test]
fn a_single_point_is_refused() {
    assert!(interpolate_curve3(&[Point3::new(1.0, 2.0, 3.0)]).is_err());
}

/// Two and three points still interpolate, at a lower degree.
///
/// Padding with invented points to reach cubic would fabricate shape, so the
/// degree drops instead.
#[test]
fn few_points_interpolate_at_a_lower_degree() {
    let two = vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)];
    let curve = interpolate_curve3(&two).expect("two points define a line");
    assert_eq!(curve.degree, 1);
    assert!((sample(&curve, 1.0) - two[1]).length() < 1e-12);

    let three = vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 2.0, 0.0),
        Point3::new(3.0, 0.0, 0.0),
    ];
    let curve = interpolate_curve3(&three).expect("three points define a quadratic");
    assert_eq!(curve.degree, 2);
    for (&t, expected) in chord_parameters(&three).iter().zip(&three) {
        assert!((sample(&curve, t) - *expected).length() < 1e-9);
    }
}
