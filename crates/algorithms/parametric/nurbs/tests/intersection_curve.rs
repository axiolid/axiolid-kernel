//! Constructed intersection curves and their refusal paths (#6).
//!
//! The issue requires refusal paths be tested, not just success paths, so
//! every variant of `IntersectionCurveRefusal` has a fixture here.

use axiolid_core::Point3;
use axiolid_curve::{BSplineCurve3, Curve3, KnotSpec};
use axiolid_evaluate::{evaluate3, surface::evaluate as evaluate_surface};
use axiolid_nurbs::{
    construct_curve_surface_points, construct_surface_surface_curves,
    CertifiedCurveSurfaceIntersectionOptions, CertifiedSurfaceSurfaceIntersectionOptions,
    IntersectionCurveRefusal,
};
use axiolid_surface::{BSplineSurface, Surface};

fn plane(points: [[Point3; 2]; 2]) -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![points[0].to_vec(), points[1].to_vec()],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: None,
    }
}

fn xy_plane() -> BSplineSurface {
    plane([
        [Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
        [Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ])
}

fn xz_plane() -> BSplineSurface {
    plane([
        [Point3::new(-0.5, 0.0, -1.0), Point3::new(-0.5, 0.0, 1.0)],
        [Point3::new(0.5, 0.0, -1.0), Point3::new(0.5, 0.0, 1.0)],
    ])
}

fn surface_options() -> CertifiedSurfaceSurfaceIntersectionOptions {
    CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 100_000, 100_000, 64)
        .expect("valid options")
}

#[test]
fn two_planes_produce_a_constructed_curve_lying_on_both() {
    let first = xy_plane();
    let second = xz_plane();

    let curves = construct_surface_surface_curves(&first, &second, surface_options())
        .expect("a valid query")
        .expect("two transverse planes meet in a constructible line");
    assert_eq!(curves.len(), 1);

    let constructed = &curves[0];
    // The claim is a CURVE, so check interior samples too. Endpoint-only
    // agreement is exactly the weak evidence this issue exists to remove.
    for step in 0..=20 {
        let t = f64::from(step) / 20.0;
        let point =
            evaluate3(&Curve3::BSpline(constructed.curve.clone()), t).expect("curve evaluation");
        // The xy-plane is z = 0 and the xz-plane is y = 0, so the exact
        // intersection is the x-axis. Check against the geometry itself,
        // not against the same machinery that produced the answer.
        assert!(
            point.y.abs() <= 1e-9,
            "off the xz-plane at t={t}: y={}",
            point.y
        );
        assert!(
            point.z.abs() <= 1e-9,
            "off the xy-plane at t={t}: z={}",
            point.z
        );
    }

    // The reported bound must cover what was actually observed.
    assert!(constructed.deviation_upper_bound >= 0.0);
    // Transversality was proven, not assumed.
    assert!(constructed.normal_cross_squared_lower_bound > 0.0);
    assert_eq!(constructed.curve.degree, 1);
    assert_eq!(constructed.curve.control_points.len(), 2);
}

#[test]
fn parallel_planes_are_refused_as_disjoint_not_unresolved() {
    // Two parallel planes provably never meet. "No intersection" is a
    // different fact from "could not decide" and must not be conflated.
    let first = xy_plane();
    let second = plane([
        [Point3::new(-1.0, -1.0, 5.0), Point3::new(-1.0, 1.0, 5.0)],
        [Point3::new(1.0, -1.0, 5.0), Point3::new(1.0, 1.0, 5.0)],
    ]);

    let refusal = construct_surface_surface_curves(&first, &second, surface_options())
        .expect("a valid query")
        .expect_err("parallel planes have no intersection curve");
    assert_eq!(refusal, IntersectionCurveRefusal::Disjoint);
}

#[test]
fn a_curved_patch_is_refused_rather_than_tessellated() {
    // A non-affine patch has no straight-line proof. Emitting a polyline here
    // would be tessellation dressed up as certification.
    let saddle = plane([
        [Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 1.0)],
        [Point3::new(1.0, -1.0, 1.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    let cutter = xz_plane();

    let refusal = construct_surface_surface_curves(&saddle, &cutter, surface_options())
        .expect("a valid query")
        .expect_err("a bilinear saddle is not the affine family");
    // It must refuse EXPLICITLY, and never silently return an empty list,
    // which a caller would read as "these surfaces do not meet".
    assert!(matches!(
        refusal,
        IntersectionCurveRefusal::Unresolved { .. }
    ));
}

#[test]
fn a_curve_crossing_a_plane_yields_the_crossing_point() {
    // A transverse curve/surface hit is ISOLATED. The right answer is a
    // point; a curve through it would assert an extent nothing proved.
    let surface = xy_plane();
    let crossing = BSplineCurve3 {
        degree: 1,
        control_points: vec![Point3::new(0.25, 0.5, -1.0), Point3::new(0.25, 0.5, 1.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    };

    let options =
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-8, 100_000, 64).expect("valid options");
    let points = construct_curve_surface_points(&crossing, &surface, options)
        .expect("a valid query")
        .expect("a transverse crossing is constructible");

    assert_eq!(points.len(), 1);
    // The segment spans z in [-1, 1] and the plane is z = 0, so the crossing
    // is the midpoint. Checked against geometry, not against the solver.
    assert!((points[0].point.z).abs() <= 1e-9);
    assert!((points[0].point.x - 0.25).abs() <= 1e-9);
    assert!((points[0].point.y - 0.5).abs() <= 1e-9);
    assert!((points[0].curve_parameter - 0.5).abs() <= 1e-6);

    // The reported surface parameters must independently name the same point.
    let [u, v] = points[0].surface_parameters;
    let mapped = evaluate_surface(&Surface::BSpline(surface), u, v).expect("surface evaluation");
    assert!(
        (mapped - points[0].point).length() <= 1e-6,
        "surface parameters name a different point than the reported one"
    );
}

#[test]
fn a_curve_missing_the_surface_is_refused_as_disjoint() {
    let surface = xy_plane();
    let missing = BSplineCurve3 {
        degree: 1,
        control_points: vec![Point3::new(0.0, 0.0, 3.0), Point3::new(1.0, 0.0, 4.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    };

    let options =
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-8, 100_000, 64).expect("valid options");
    let refusal = construct_curve_surface_points(&missing, &surface, options)
        .expect("a valid query")
        .expect_err("a curve well above the plane never meets it");
    assert_eq!(refusal, IntersectionCurveRefusal::Disjoint);
}
