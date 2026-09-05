//! Point-based (Cartesian) trims must resolve to curve parameters.
//!
//! A trim selector naming a POINT is the only way some exchange formats can
//! state where a curve starts and ends: a three-point arc knows its endpoints
//! but not their parameters. Accepting such a curve into the graph and then
//! refusing it at compile time makes the geometry representable but not
//! usable, which is the worst of both.

use axiolid_core::{Point2, Point3, Scalar, Tolerance, Vec2, Vec3};
use axiolid_curve::{Circle2, Circle3, Curve2, Curve3, Line3};
use axiolid_evaluate::curve::{invert2, invert3};

fn tol() -> Tolerance {
    Tolerance::MILLIMETRE
}

/// A point on a line inverts to the parameter that reproduces it.
#[test]
fn a_point_on_a_line_inverts_to_its_parameter() {
    let line = Curve3::Line(Line3 {
        origin: Point3::ZERO,
        direction: Vec3::X,
    });
    let t = invert3(&line, Point3::new(4.0, 0.0, 0.0), tol()).expect("a point on the line inverts");
    assert!((t - 4.0).abs() < 1e-12, "expected parameter 4, got {t}");
}

/// A point off the curve is refused rather than snapped to the nearest one.
///
/// Silently projecting would answer a question the caller did not ask and
/// would hide a genuinely inconsistent model.
#[test]
fn a_point_off_the_curve_is_refused() {
    let line = Curve3::Line(Line3 {
        origin: Point3::ZERO,
        direction: Vec3::X,
    });
    assert!(
        invert3(&line, Point3::new(4.0, 5.0, 0.0), tol()).is_err(),
        "a point 5 units off the line must not silently project onto it"
    );
}

/// The 2D arc case: an IfcArcIndex p-curve trims a circle by its endpoints.
#[test]
fn a_point_on_a_two_dimensional_circle_inverts_to_its_angle() {
    let circle = Curve2::Circle(Circle2 {
        frame: axiolid_core::Frame2 {
            origin: Point2::ZERO,
            x: Vec2::X,
            y: Vec2::Y,
        },
        radius: 2.0,
    });
    let quarter = invert2(&circle, Point2::new(0.0, 2.0), tol()).expect("a point on the circle");
    let expected = std::f64::consts::FRAC_PI_2;
    assert!(
        (quarter - expected).abs() < 1e-12,
        "expected {expected}, got {quarter}"
    );
}

/// Inversion is the left inverse of evaluation, across the whole domain.
#[test]
fn inversion_round_trips_through_evaluation() {
    let circle = Curve3::Circle(Circle3 {
        frame: axiolid_core::Frame3 {
            origin: Point3::new(1.0, -2.0, 0.5),
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        },
        radius: 3.0,
    });
    for step in 0..8 {
        let t = Scalar::from(step) * std::f64::consts::TAU / 8.0;
        let point = axiolid_evaluate::curve::evaluate3(&circle, t).expect("evaluates");
        let back = invert3(&circle, point, tol()).expect("inverts");
        let again = axiolid_evaluate::curve::evaluate3(&circle, back).expect("re-evaluates");
        assert!(
            (again - point).length() < 1e-9,
            "round trip drifted at t={t}: {point:?} -> {back} -> {again:?}"
        );
    }
}

/// A B-spline basis has no closed-form inversion and must refuse by name.
///
/// Iterating here would introduce a tolerance and a convergence failure mode
/// into trim resolution. A bounded refusal keeps the compile path exact and
/// leaves certified iteration to a caller that can carry its evidence.
#[test]
fn a_bspline_basis_is_refused_rather_than_iterated() {
    use axiolid_curve::{BSplineCurve, KnotSpec};
    let spline = Curve3::BSpline(BSplineCurve {
        degree: 1,
        control_points: vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed: false,
        self_intersect: None,
        knot_spec: KnotSpec::Unspecified,
    });
    let error = invert3(&spline, Point3::new(0.5, 0.0, 0.0), tol())
        .expect_err("a b-spline basis has no closed-form inversion");
    let text = format!("{error:?}");
    assert!(
        text.contains("closed form") || text.contains("Unsupported"),
        "the refusal must name why it refused, got: {text}"
    );
}

/// A point off a CIRCLE is refused, not snapped to the nearest angle.
///
/// This exercises a different guard than the line case: the conic arctangent
/// always yields an angle, whatever the radius, so a point at the right
/// bearing but the wrong distance inverts to a plausible-looking parameter.
/// Only re-evaluating and measuring the residual catches it.
#[test]
fn a_point_off_a_circle_is_refused_despite_a_valid_angle() {
    let circle = Curve3::Circle(Circle3 {
        frame: axiolid_core::Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        },
        radius: 2.0,
    });
    // Same bearing as t=0, but at radius 5 instead of 2.
    let error = invert3(&circle, Point3::new(5.0, 0.0, 0.0), tol())
        .expect_err("a point at the wrong radius is not on the circle");
    let text = format!("{error:?}");
    assert!(
        text.contains("from the curve"),
        "the refusal must report the residual, got: {text}"
    );
}

/// The 2D equivalent, guarding the p-curve arc path.
#[test]
fn a_point_off_a_two_dimensional_circle_is_refused() {
    let circle = Curve2::Circle(Circle2 {
        frame: axiolid_core::Frame2 {
            origin: Point2::ZERO,
            x: Vec2::X,
            y: Vec2::Y,
        },
        radius: 1.0,
    });
    assert!(
        invert2(&circle, Point2::new(0.0, 4.0), tol()).is_err(),
        "a point at 4x the radius must not invert to a valid angle"
    );
}
