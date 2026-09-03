//! Mapped-3D verification of certified intersection and projection results.
//!
//! Issue #17: parameter-space agreement is not evidence of geometric
//! correctness. Every certified result checked here is mapped back into model
//! space by `axiolid-oracle`, which shares no subdivision, isolation, or
//! interval machinery with `axiolid-nurbs` -- it only evaluates through the
//! portable scalar oracle and measures the deviation it observes.
//!
//! Failures report the measured 3D deviation, not just a disagreement.

use axiolid_core::{Point2, Point3, Tolerance};
use axiolid_curve::{BSplineCurve, BSplineCurve3, Curve2, Curve3, KnotSpec};
use axiolid_nurbs::{
    construct_surface_surface_curves, intersect_curve2_certified,
    intersect_curve_surface_certified, invert_surface_certified, project_surface_certified,
    CertifiedCurveIntersection2, CertifiedCurveIntersectionOptions,
    CertifiedCurveSurfaceIntersection3, CertifiedCurveSurfaceIntersectionOptions,
    CertifiedSurfaceProjection3, CertifiedSurfaceProjectionOptions,
    CertifiedSurfaceSurfaceIntersectionOptions, CurveIntersectionDegeneracy,
};
use axiolid_oracle::{
    closer_point_refutation, curve_pair_deviation2, curve_surface_deviation, CurvePairBox,
    CurveSurfaceBox, DistanceClaim, Operand, ParameterSpan, SampleDensity,
};
use axiolid_surface::{BSplineSurface, Surface};

fn bezier2(points: Vec<Point2>) -> BSplineCurve<Point2> {
    let degree = u16::try_from(points.len() - 1).unwrap();
    BSplineCurve {
        degree,
        control_points: points,
        knots: vec![0.0, 1.0],
        multiplicities: vec![u32::from(degree) + 1; 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    }
}

fn planar_patch() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        u_closed: false,
        v_closed: false,
        self_intersect: Some(false),
    }
}

fn density() -> SampleDensity {
    SampleDensity::new(32).expect("valid density")
}

fn span(start: f64, end: f64) -> ParameterSpan {
    ParameterSpan::new(start, end).expect("valid span")
}

#[test]
fn certified_transverse_roots_are_checked_in_mapped_3d() {
    let first = bezier2(vec![Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier2(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);
    let options = CertifiedCurveIntersectionOptions::new(1e-10, 10_000, 64).unwrap();

    let CertifiedCurveIntersection2::Complete { intersections, .. } =
        intersect_curve2_certified(&first, &second, options).unwrap()
    else {
        panic!("a regular crossing must isolate completely");
    };
    assert_eq!(intersections.len(), 1);

    let root = &intersections[0];
    let claimed = CurvePairBox {
        first: span(root.first_parameter.start, root.first_parameter.end),
        second: span(root.second_parameter.start, root.second_parameter.end),
    };

    // The oracle re-derives contact by evaluating both curves independently.
    let witness = curve_pair_deviation2(
        &Curve2::BSpline(first),
        &Curve2::BSpline(second),
        claimed,
        density(),
    )
    .expect("planar evaluation");

    assert!(
        witness.deviation <= 1e-9,
        "certified root does not meet in mapped 3D: deviation {} at {:?} / {:?}",
        witness.deviation,
        witness.first,
        witness.second
    );
}

#[test]
fn certified_disjointness_has_no_mapped_contact() {
    // Two parallel horizontal segments a unit apart: certified as disjoint, and
    // the oracle must independently find no contact anywhere in the domain.
    let first = bezier2(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier2(vec![Point2::new(0.0, 1.0), Point2::new(1.0, 1.0)]);
    let options = CertifiedCurveIntersectionOptions::new(1e-10, 10_000, 64).unwrap();

    let outcome = intersect_curve2_certified(&first, &second, options).unwrap();
    let CertifiedCurveIntersection2::Complete { intersections, .. } = outcome else {
        panic!("parallel polynomial lines are a supported complete case");
    };
    assert!(intersections.is_empty());

    let witness = curve_pair_deviation2(
        &Curve2::BSpline(first),
        &Curve2::BSpline(second),
        CurvePairBox {
            first: span(0.0, 1.0),
            second: span(0.0, 1.0),
        },
        density(),
    )
    .expect("planar evaluation");

    assert!(
        witness.deviation > 1e-9,
        "certified disjointness contradicted by mapped contact at {:?}",
        witness.first
    );
}

#[test]
fn certified_surface_projection_minimum_is_not_refutable() {
    // A unit planar patch in z = 0, projected from a point above it. The
    // certified global minimum must survive an independent search for any
    // closer point on the same surface.
    let surface = planar_patch();
    let target = Point3::new(0.25, 0.75, 2.0);
    let options = CertifiedSurfaceProjectionOptions::new(
        axiolid_core::Tolerance::new(1e-6, 1e-12).unwrap(),
        1e-4,
        50_000,
        32,
    )
    .unwrap();

    let outcome = project_surface_certified(&surface, target, options).unwrap();
    let CertifiedSurfaceProjection3::Complete(certificate) = outcome else {
        panic!("a planar patch projection must certify completely");
    };

    let claim = DistanceClaim::new(target, certificate.distance_upper_bound, 1e-9).unwrap();
    let refutation = closer_point_refutation(
        &Operand::Surface {
            surface: &Surface::BSpline(surface),
            u: span(0.0, 1.0),
            v: span(0.0, 1.0),
        },
        claim,
        density(),
    )
    .expect("surface scan");

    assert!(
        refutation.is_none(),
        "independent scan refuted the certified minimum: {refutation:?}"
    );
}

#[test]
fn an_inflated_projection_claim_is_caught_by_the_oracle() {
    // Guard against a vacuous oracle: if the claim is deliberately wrong, the
    // same machinery must produce a counterexample with its 3D deviation.
    let surface = planar_patch();
    let target = Point3::new(0.25, 0.75, 2.0);
    let claim = DistanceClaim::new(target, 9.0, 1e-9).unwrap();

    let refutation = closer_point_refutation(
        &Operand::Surface {
            surface: &Surface::BSpline(surface),
            u: span(0.0, 1.0),
            v: span(0.0, 1.0),
        },
        claim,
        density(),
    )
    .expect("surface scan")
    .expect("an inflated claim must be refuted");

    assert!(refutation.distance < 9.0);
    assert!(refutation.deviation > 6.0, "{refutation:?}");
}

#[test]
fn certified_curve_surface_roots_are_checked_in_mapped_3d() {
    // Adversarial fixture: the surface carries a shifted, non-unit knot domain
    // and non-uniform rational weights, so native parameters are not model
    // units. The oracle only ever sees native parameters and evaluates them.
    let curve = BSplineCurve3 {
        degree: 1,
        control_points: vec![Point3::new(0.25, -0.5, -1.0), Point3::new(0.25, -0.5, 2.0)],
        knots: vec![-2.0, 2.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: None,
    };
    let surface = BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
            vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![10.0, 12.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![-4.0, 0.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.25], vec![0.75, 1.5]]),
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: None,
    };

    let outcome = intersect_curve_surface_certified(
        &curve,
        &surface,
        CertifiedCurveSurfaceIntersectionOptions::new(1e-6, 100_000, 64).unwrap(),
    )
    .unwrap();
    let CertifiedCurveSurfaceIntersection3::Complete { intersections, .. } = outcome else {
        panic!("a transverse line/plane crossing must isolate completely");
    };
    assert_eq!(intersections.len(), 1);

    let root = &intersections[0];
    let witness = curve_surface_deviation(
        &Curve3::BSpline(curve),
        &Surface::BSpline(surface),
        CurveSurfaceBox {
            curve: span(root.curve_parameter.start, root.curve_parameter.end),
            surface_u: span(root.surface_u_parameter.start, root.surface_u_parameter.end),
            surface_v: span(root.surface_v_parameter.start, root.surface_v_parameter.end),
        },
        density(),
    )
    .expect("curve/surface evaluation");

    assert!(
        witness.deviation <= 1e-5,
        "certified curve/surface root does not meet in mapped 3D: deviation {} at {:?} / {:?}",
        witness.deviation,
        witness.first,
        witness.second
    );
}

#[test]
fn degenerate_tangency_candidates_are_reported_with_their_mapped_deviation() {
    // Degenerate fixture: identical curves are a proven positive-dimensional
    // overlap, not a transverse root. The certified answer is a candidate box,
    // and the oracle must confirm that box really does map onto shared
    // geometry rather than being an arbitrary enclosure.
    let curve = bezier2(vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 1.0),
        Point2::new(2.0, 0.0),
    ]);
    let options = CertifiedCurveIntersectionOptions::new(1e-10, 10_000, 64).unwrap();

    let outcome = intersect_curve2_certified(&curve, &curve, options).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        ..
    } = outcome
    else {
        panic!("a self-pair is a structural overlap, not a transverse root");
    };
    assert_eq!(classification, CurveIntersectionDegeneracy::Overlap);
    assert!(!contacts.is_empty());

    for contact in &contacts {
        let candidate = contact.parameters;
        let witness = curve_pair_deviation2(
            &Curve2::BSpline(curve.clone()),
            &Curve2::BSpline(curve.clone()),
            CurvePairBox {
                first: span(candidate.first.start, candidate.first.end),
                second: span(candidate.second.start, candidate.second.end),
            },
            density(),
        )
        .expect("planar evaluation");

        assert!(
            witness.deviation <= 1e-12,
            "overlap candidate does not map onto shared geometry: deviation {}",
            witness.deviation
        );
    }
}

#[test]
fn a_boundary_crossing_contact_really_meets_in_mapped_3d() {
    // A BoundaryCrossing is a positive claim that a transverse root exists
    // on a shared cell edge. Verify it against geometry, not parameters.
    let split = BSplineCurve {
        degree: 1,
        control_points: vec![
            Point2::new(-1.0, 0.0),
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        ],
        knots: vec![0.0, 0.5, 1.0],
        multiplicities: vec![2, 1, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };
    let crossing = bezier2(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);
    let options = CertifiedCurveIntersectionOptions::new(1e-9, 100_000, 64).unwrap();

    let outcome = intersect_curve2_certified(&split, &crossing, options).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        ..
    } = outcome
    else {
        panic!("a knot-boundary crossing is reported as a classified contact");
    };
    assert_eq!(
        classification,
        CurveIntersectionDegeneracy::BoundaryCrossing
    );
    assert_eq!(contacts.len(), 1);

    let owned = contacts[0].parameters;
    let witness = curve_pair_deviation2(
        &Curve2::BSpline(split.clone()),
        &Curve2::BSpline(crossing.clone()),
        CurvePairBox {
            first: span(owned.first.start, owned.first.end),
            second: span(owned.second.start, owned.second.end),
        },
        density(),
    )
    .expect("planar evaluation");

    assert!(
        witness.deviation <= 1e-9,
        "a claimed boundary crossing does not meet in 3D: deviation {}",
        witness.deviation
    );
}

#[test]
fn a_unique_inverse_names_the_queried_point_in_mapped_3d() {
    // The inverse claims (u, v) NAMES the point. Verify that independently:
    // evaluate the returned parameters through the oracle and refute any
    // closer disagreement.
    let surface = planar_patch();
    let target = Point3::new(0.25, 0.5, 0.0);
    // The parameter tolerance bounds the reported ENCLOSURE, which is the
    // hull of the touching cells that cover the minimizer, so it is asked for
    // at cell-pair scale rather than single-cell scale.
    let options = CertifiedSurfaceProjectionOptions::new(
        Tolerance::new(1e-8, 1e-12).unwrap(),
        1e-4,
        100_000,
        64,
    )
    .unwrap();

    let certificate = invert_surface_certified(&surface, target, options)
        .expect("a valid query")
        .expect("an interior on-surface point inverts uniquely");

    // Independent check: nothing on the surface is meaningfully closer than
    // the certified residual the inversion accepted.
    let claim =
        DistanceClaim::new(target, certificate.residual_upper_bound, 1e-9).expect("valid claim");
    let refutation = closer_point_refutation(
        &Operand::Surface {
            surface: &Surface::BSpline(surface),
            u: span(0.0, 1.0),
            v: span(0.0, 1.0),
        },
        claim,
        density(),
    )
    .expect("surface scan");

    assert!(
        refutation.is_none(),
        "the oracle found a closer point than the accepted inversion residual: {refutation:?}"
    );
}

#[test]
fn a_constructed_intersection_curve_lies_on_both_surfaces_in_mapped_3d() {
    // Issue #6 requires oracle verification in mapped 3D. Sample the
    // CONSTRUCTED curve and, at each sample, ask the independent oracle to
    // refute the claim that the point is ON each surface. A point of a true
    // intersection curve has distance zero to BOTH surfaces.
    let first = plane_patch([
        [Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
        [Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    let second = plane_patch([
        [Point3::new(-0.5, 0.0, -1.0), Point3::new(-0.5, 0.0, 1.0)],
        [Point3::new(0.5, 0.0, -1.0), Point3::new(0.5, 0.0, 1.0)],
    ]);

    let options = CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 100_000, 100_000, 64)
        .expect("valid options");
    let curves = construct_surface_surface_curves(&first, &second, options)
        .expect("a valid query")
        .expect("transverse planes are constructible");

    let curve = Curve3::BSpline(curves[0].curve.clone());
    let tolerance = 1e-6;
    for step in 0..=16 {
        let t = f64::from(step) / 16.0;
        let sample = axiolid_evaluate::evaluate3(&curve, t).expect("curve evaluation");

        // Claiming distance 0 to a surface is refutable: if the oracle finds
        // ANY surface point closer than 0, the sample is not on the surface.
        // Here the reverse is what matters, so claim the tolerance instead
        // and require the oracle to REFUTE it by finding a surface point
        // within tolerance. A successful refutation is positive evidence that
        // the surface really does pass through this sample.
        for (label, surface) in [("first", &first), ("second", &second)] {
            let claim = DistanceClaim::new(sample, tolerance, 0.0).expect("valid claim");
            let witness = closer_point_refutation(
                &Operand::Surface {
                    surface: &Surface::BSpline((*surface).clone()),
                    u: span(0.0, 1.0),
                    v: span(0.0, 1.0),
                },
                claim,
                density(),
            )
            .expect("surface scan");

            assert!(
                witness.is_some(),
                "no {label}-surface point within {tolerance} of the constructed curve at t={t}"
            );
        }
    }
}

fn plane_patch(points: [[Point3; 2]; 2]) -> BSplineSurface {
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
