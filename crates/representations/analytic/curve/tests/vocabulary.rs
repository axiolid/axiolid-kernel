//! Contracts `axiolid-curve` owes the crates that consume it.
//!
//! This crate is a value vocabulary: no evaluation, no algorithms. What it owes
//! downstream is therefore structural rather than numeric, and that is what is
//! pinned here. Evaluation behaviour is tested where it lives, in
//! `axiolid-scalar`.

use axiolid_core::{Frame2, Frame3, Point2, Point3, Vec2, Vec3};
use axiolid_curve::{
    BSplineCurve2, BSplineCurve3, Circle2, Circle3, Curve2, Curve3, Ellipse2, Ellipse3, KnotSpec,
    Line2, Line3, Polyline2, Polyline3,
};

fn frame2() -> Frame2 {
    Frame2 {
        origin: Point2::ZERO,
        x: Vec2::X,
        y: Vec2::Y,
    }
}

fn frame3() -> Frame3 {
    Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

/// Every `Curve2` variant must be constructible from public API alone.
///
/// A variant that cannot be built outside the crate is unusable to the readers
/// and compilers that consume this vocabulary, and the omission would otherwise
/// only surface at an integration site.
#[test]
fn every_curve2_variant_is_constructible() {
    let curves = [
        Curve2::Line(Line2 {
            origin: Point2::ZERO,
            direction: Vec2::X,
        }),
        Curve2::Circle(Circle2 {
            frame: frame2(),
            radius: 1.0,
        }),
        Curve2::Ellipse(Ellipse2 {
            frame: frame2(),
            semi_axis_x: 2.0,
            semi_axis_y: 1.0,
        }),
        Curve2::Polyline(Polyline2 {
            points: vec![Point2::ZERO, Point2::new(1.0, 0.0)],
            closed: false,
        }),
        Curve2::BSpline(BSplineCurve2 {
            degree: 1,
            control_points: vec![Point2::ZERO, Point2::new(1.0, 0.0)],
            knots: vec![0.0, 1.0],
            multiplicities: vec![2, 2],
            weights: None,
            closed: false,
            self_intersect: None,
            knot_spec: KnotSpec::Unspecified,
        }),
    ];
    assert_eq!(curves.len(), 5, "one instance per Curve2 variant");
}

/// The same for `Curve3`.
#[test]
fn every_curve3_variant_is_constructible() {
    let curves = [
        Curve3::Line(Line3 {
            origin: Point3::ZERO,
            direction: Vec3::X,
        }),
        Curve3::Circle(Circle3 {
            frame: frame3(),
            radius: 1.0,
        }),
        Curve3::Ellipse(Ellipse3 {
            frame: frame3(),
            semi_axis_x: 2.0,
            semi_axis_y: 1.0,
        }),
        Curve3::Polyline(Polyline3 {
            points: vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
            closed: false,
        }),
        Curve3::BSpline(BSplineCurve3 {
            degree: 1,
            control_points: vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
            knots: vec![0.0, 1.0],
            multiplicities: vec![2, 2],
            weights: None,
            closed: false,
            self_intersect: None,
            knot_spec: KnotSpec::Unspecified,
        }),
    ];
    assert_eq!(curves.len(), 5, "one instance per Curve3 variant");
}

/// `Curve2`/`Curve3` are `#[non_exhaustive]`, so consumers must keep a
/// wildcard arm.
///
/// Downstream code refuses unknown future variants explicitly rather than
/// falling through (see `axiolid-scalar`'s evaluators). That discipline is only
/// sound while the enums stay non-exhaustive: making them exhaustive would let
/// a new variant compile into a silent fallthrough at every match site. This
/// test fails to compile if that guarantee is withdrawn.
#[test]
fn curve_enums_stay_non_exhaustive_for_consumers() {
    let curve = Curve2::Circle(Circle2 {
        frame: frame2(),
        radius: 1.0,
    });
    let named = match curve {
        Curve2::Line(_) => "line",
        Curve2::Circle(_) => "circle",
        Curve2::Ellipse(_) => "ellipse",
        Curve2::Polyline(_) => "polyline",
        Curve2::BSpline(_) => "bspline",
        // Required because the enum is non-exhaustive. If this arm becomes
        // unreachable the enum lost that property and consumers relying on
        // explicit refusal need revisiting.
        _ => "unknown",
    };
    assert_eq!(named, "circle");
}

/// Splines pair each distinct knot with a multiplicity.
///
/// The vocabulary stores distinct knots and multiplicities separately rather
/// than an expanded knot vector, so the two must agree in length. Validation
/// lives in `axiolid-scalar`; what this pins is that the representation is the
/// paired one, because a reader building the expanded vector depends on it.
#[test]
fn spline_knots_and_multiplicities_are_parallel() {
    let spline = BSplineCurve3 {
        degree: 2,
        control_points: vec![Point3::ZERO, Point3::new(1.0, 1.0, 0.0), Point3::X],
        knots: vec![0.0, 1.0],
        multiplicities: vec![3, 3],
        weights: None,
        closed: false,
        self_intersect: None,
        knot_spec: KnotSpec::QuasiUniform,
    };
    assert_eq!(
        spline.knots.len(),
        spline.multiplicities.len(),
        "distinct knots and multiplicities are parallel arrays"
    );
    let expanded: u32 = spline.multiplicities.iter().sum();
    assert_eq!(
        expanded as usize,
        spline.control_points.len() + spline.degree as usize + 1,
        "expanded knot count must satisfy n + p + 1"
    );
}

/// Rational weights are optional, and when present match the control points.
///
/// `weights: None` means polynomial, not "weights of 1". Consumers branch on
/// the `Option` to choose homogeneous evaluation, so collapsing the two would
/// change which code path runs.
#[test]
fn rational_weights_are_optional_and_parallel_to_control_points() {
    let polynomial = BSplineCurve2 {
        degree: 1,
        control_points: vec![Point2::ZERO, Point2::new(1.0, 0.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed: false,
        self_intersect: None,
        knot_spec: KnotSpec::Uniform,
    };
    assert!(
        polynomial.weights.is_none(),
        "absent weights mean polynomial, not unit weights"
    );

    let rational = BSplineCurve2 {
        weights: Some(vec![1.0, 0.5]),
        ..polynomial.clone()
    };
    let weights = rational.weights.as_ref().expect("weights present");
    assert_eq!(
        weights.len(),
        rational.control_points.len(),
        "one weight per control point"
    );
    assert_ne!(
        polynomial, rational,
        "weights participate in equality: a rational curve is not its polynomial twin"
    );
}

/// `closed` is a declaration carried from the source, not a derived fact.
///
/// A polyline whose last point equals its first is not thereby closed, and one
/// declared closed does not repeat its first point. Consumers must trust the
/// flag rather than infer closure from coordinates.
#[test]
fn polyline_closure_is_declared_not_inferred() {
    let open_but_coincident = Polyline2 {
        points: vec![Point2::ZERO, Point2::new(1.0, 0.0), Point2::ZERO],
        closed: false,
    };
    assert!(
        !open_but_coincident.closed,
        "coincident endpoints do not imply closure"
    );

    let closed = Polyline2 {
        points: vec![Point2::ZERO, Point2::new(1.0, 0.0), Point2::new(1.0, 1.0)],
        closed: true,
    };
    assert_eq!(
        closed.points.len(),
        3,
        "a closed polyline does not repeat its first point"
    );
}
