//! Validation of the mapped-3D verification oracle itself.
//!
//! An oracle nobody checked is just a second opinion (ADR 0012). These tests
//! pin the oracle to values computed independently of any intersection or
//! projection implementation: analytic contact points, closed-form distances,
//! and deliberately planted counterexamples.

use axiolid_core::{Frame3, Point2, Point3, Vec3};
use axiolid_curve::{Curve2, Curve3, Line2, Line3};
use axiolid_oracle::{
    closer_point_refutation, contact_witness, curve_pair_deviation2, curve_pair_deviation3,
    curve_surface_deviation, surface_pair_deviation, CurvePairBox, CurveSurfaceBox, DistanceClaim,
    Operand, ParameterSpan, SampleDensity, SurfacePairBox,
};
use axiolid_surface::{Plane, Surface};

fn density(steps: u32) -> SampleDensity {
    SampleDensity::new(steps).expect("valid density")
}

fn span(start: f64, end: f64) -> ParameterSpan {
    ParameterSpan::new(start, end).expect("valid span")
}

fn line2(origin: Point2, direction: Point2) -> Curve2 {
    Curve2::Line(Line2 {
        origin,
        direction: axiolid_core::Vec2::new(direction.x, direction.y),
    })
}

fn line3(origin: Point3, direction: Vec3) -> Curve3 {
    Curve3::Line(Line3 { origin, direction })
}

fn plane_at(z: f64) -> Surface {
    Surface::Plane(Plane {
        frame: Frame3 {
            origin: Point3::new(0.0, 0.0, z),
            x: Vec3::new(1.0, 0.0, 0.0),
            y: Vec3::new(0.0, 1.0, 0.0),
            z: Vec3::new(0.0, 0.0, 1.0),
        },
    })
}

#[test]
fn spans_and_densities_reject_vacuous_policy() {
    assert!(ParameterSpan::new(1.0, 0.0).is_err());
    assert!(ParameterSpan::new(f64::NAN, 1.0).is_err());
    assert!(ParameterSpan::new(0.0, f64::INFINITY).is_err());
    assert!(SampleDensity::new(0).is_err());
    assert!(SampleDensity::new(4097).is_err());
    assert_eq!(density(4).positions(), 5);
}

#[test]
fn crossing_planar_lines_agree_at_the_analytic_root() {
    // x-axis and y-axis cross exactly at the origin; both roots are at t = 0.
    let first = line2(Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0));
    let second = line2(Point2::new(0.0, -1.0), Point2::new(0.0, 1.0));

    let found = curve_pair_deviation2(
        &first,
        &second,
        CurvePairBox {
            first: span(0.0, 2.0),
            second: span(0.0, 2.0),
        },
        density(8),
    )
    .expect("planar evaluation");

    assert!(contact_witness(found) < 1e-12, "{found:?}");
    assert!((found.first - Point3::new(0.0, 0.0, 0.0)).length() < 1e-12);
}

#[test]
fn parallel_planar_lines_report_their_true_separation() {
    // Two horizontal lines one unit apart never meet: the oracle must report
    // the real gap rather than an optimistic zero.
    let first = line2(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0));
    let second = line2(Point2::new(0.0, 1.0), Point2::new(1.0, 0.0));

    let found = curve_pair_deviation2(
        &first,
        &second,
        CurvePairBox {
            first: span(0.0, 1.0),
            second: span(0.0, 1.0),
        },
        density(16),
    )
    .expect("planar evaluation");

    assert!((contact_witness(found) - 1.0).abs() < 1e-12, "{found:?}");
}

#[test]
fn skew_spatial_lines_report_the_closed_form_common_perpendicular() {
    // Classic skew pair: the x-axis and a y-directed line offset in z. Their
    // minimum separation is exactly the z offset.
    let first = line3(Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let second = line3(Point3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 1.0, 0.0));

    let found = curve_pair_deviation3(
        &first,
        &second,
        CurvePairBox {
            first: span(-1.0, 1.0),
            second: span(-1.0, 1.0),
        },
        density(8),
    )
    .expect("spatial evaluation");

    assert!((contact_witness(found) - 3.0).abs() < 1e-12, "{found:?}");
}

#[test]
fn curve_meeting_a_plane_agrees_where_it_pierces() {
    // A vertical line pierces z = 2 at t = 2; the sampled grid includes it.
    let curve = line3(Point3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    let surface = plane_at(2.0);

    let found = curve_surface_deviation(
        &curve,
        &surface,
        CurveSurfaceBox {
            curve: span(0.0, 4.0),
            surface_u: span(-1.0, 1.0),
            surface_v: span(-1.0, 1.0),
        },
        density(4),
    )
    .expect("curve/surface evaluation");

    assert!(contact_witness(found) < 1e-12, "{found:?}");
}

#[test]
fn separated_planes_report_their_offset_not_contact() {
    let found = surface_pair_deviation(
        &plane_at(0.0),
        &plane_at(5.0),
        SurfacePairBox {
            first_u: span(-1.0, 1.0),
            first_v: span(-1.0, 1.0),
            second_u: span(-1.0, 1.0),
            second_v: span(-1.0, 1.0),
        },
        density(4),
    )
    .expect("surface pair evaluation");

    assert!((contact_witness(found) - 5.0).abs() < 1e-12, "{found:?}");
}

#[test]
fn an_overstated_minimum_distance_is_refuted_with_a_witness() {
    // The true distance from (0,0,0) to the plane z = 1 is 1. A claim of 4 is
    // false, and the oracle must produce the closer point that proves it.
    let surface = plane_at(1.0);
    let claim = DistanceClaim::new(Point3::new(0.0, 0.0, 0.0), 4.0, 1e-9).expect("valid claim");

    let refutation = closer_point_refutation(
        &Operand::Surface {
            surface: &surface,
            u: span(-1.0, 1.0),
            v: span(-1.0, 1.0),
        },
        claim,
        density(4),
    )
    .expect("surface scan")
    .expect("an overstated claim must be refuted");

    assert!((refutation.distance - 1.0).abs() < 1e-12, "{refutation:?}");
    assert!((refutation.deviation - 3.0).abs() < 1e-12, "{refutation:?}");
}

#[test]
fn a_correct_minimum_distance_is_not_refuted() {
    // Same geometry, honest claim: nothing on the plane is closer than 1.
    let surface = plane_at(1.0);
    let claim = DistanceClaim::new(Point3::new(0.0, 0.0, 0.0), 1.0, 1e-9).expect("valid claim");

    let refutation = closer_point_refutation(
        &Operand::Surface {
            surface: &surface,
            u: span(-1.0, 1.0),
            v: span(-1.0, 1.0),
        },
        claim,
        density(8),
    )
    .expect("surface scan");

    assert!(refutation.is_none(), "{refutation:?}");
}

#[test]
fn curve_distance_claims_are_refuted_on_the_curve_too() {
    let curve = line3(Point3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let claim = DistanceClaim::new(Point3::new(0.5, 0.0, 0.0), 0.4, 0.0).expect("valid claim");

    let refutation = closer_point_refutation(
        &Operand::Curve {
            curve: &curve,
            span: span(0.0, 1.0),
        },
        claim,
        density(2),
    )
    .expect("curve scan")
    .expect("the target lies on the curve, so 0.4 is refutable");

    assert!(refutation.distance < 1e-12, "{refutation:?}");
}

#[test]
fn distance_claims_reject_nonfinite_or_negative_policy() {
    let target = Point3::new(0.0, 0.0, 0.0);
    assert!(DistanceClaim::new(target, -1.0, 0.0).is_err());
    assert!(DistanceClaim::new(target, f64::NAN, 0.0).is_err());
    assert!(DistanceClaim::new(target, 1.0, -1.0).is_err());
    assert!(DistanceClaim::new(Point3::new(f64::INFINITY, 0.0, 0.0), 1.0, 0.0).is_err());
}
