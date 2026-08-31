use axiolid_core::Point2;
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_kernel::GeomError;
use axiolid_nurbs::{
    intersect_curve2_certified, CertifiedCurveIntersection2, CertifiedCurveIntersectionOptions,
    CurveIntersectionDegeneracy, TransverseCurveIntersection2,
};

fn bezier(points: Vec<Point2>) -> BSplineCurve<Point2> {
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

fn options(max_nodes: u32) -> CertifiedCurveIntersectionOptions {
    CertifiedCurveIntersectionOptions::new(1e-10, max_nodes, 64).unwrap()
}

fn assert_resolved_to_policy(root: &TransverseCurveIntersection2) {
    assert!(root.first_parameter.end - root.first_parameter.start <= 1e-10);
    assert!(root.second_parameter.end - root.second_parameter.start <= 1e-10);
}

#[test]
fn intersection_options_reject_vacuous_parameter_resolution() {
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            CertifiedCurveIntersectionOptions::new(tolerance, 1, 1),
            Err(GeomError::InvalidInput(_))
        ));
    }
    assert!(CertifiedCurveIntersectionOptions::new(1e-6, 0, 1).is_err());
    assert!(CertifiedCurveIntersectionOptions::new(1e-6, 1, 0).is_err());
    assert!(CertifiedCurveIntersectionOptions::new(1e-6, 100_001, 1).is_err());
    assert!(CertifiedCurveIntersectionOptions::new(1e-6, 1, 65).is_err());
}

#[test]
fn certifies_one_transverse_planar_root() {
    let first = bezier(vec![Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);

    let outcome = intersect_curve2_certified(&first, &second, options(10_000)).unwrap();
    let CertifiedCurveIntersection2::Complete {
        intersections,
        visited_nodes,
    } = outcome
    else {
        panic!("a regular crossing must be completely isolated");
    };

    assert_eq!(intersections.len(), 1);
    let root = &intersections[0];
    assert_resolved_to_policy(root);
    assert!(root.first_parameter.start <= 0.5 && root.first_parameter.end >= 0.5);
    assert!(root.second_parameter.start <= 0.5 && root.second_parameter.end >= 0.5);
    assert!(root.residual_upper_bound <= 1e-10);
    assert!(root.jacobian_determinant_lower_bound > 0.0);
    assert!(visited_nodes > 0);
}

#[test]
fn certifies_a_transverse_root_on_a_nonlinear_curve() {
    let parabola = bezier(vec![
        Point2::new(-1.0, 0.25),
        Point2::new(0.0, -0.25),
        Point2::new(1.0, 0.25),
    ]);
    let vertical = bezier(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);

    let outcome = intersect_curve2_certified(&parabola, &vertical, options(100_000)).unwrap();
    let CertifiedCurveIntersection2::Complete { intersections, .. } = outcome else {
        panic!("a regular nonlinear crossing must be isolated: {outcome:?}");
    };
    assert_eq!(intersections.len(), 1);
    assert_resolved_to_policy(&intersections[0]);
    assert!(intersections[0].first_parameter.start <= 0.5);
    assert!(intersections[0].first_parameter.end >= 0.5);
    assert!(intersections[0].jacobian_determinant_lower_bound > 0.0);
}

#[test]
fn certifies_a_transverse_root_on_a_rational_quarter_circle() {
    let circle = BSplineCurve {
        degree: 2,
        control_points: vec![
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![3, 3],
        weights: Some(vec![1.0, 0.5_f64.sqrt(), 1.0]),
        closed: false,
        self_intersect: Some(false),
        knot_spec: KnotSpec::PiecewiseBezier,
    };
    let diagonal = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.5)]);

    let outcome = intersect_curve2_certified(&circle, &diagonal, options(100_000)).unwrap();
    let CertifiedCurveIntersection2::Complete { intersections, .. } = outcome else {
        panic!("the rational transverse crossing must be isolated");
    };
    assert_eq!(intersections.len(), 1);
    assert_resolved_to_policy(&intersections[0]);
    assert!(intersections[0].jacobian_determinant_lower_bound > 0.0);
}

#[test]
fn singular_or_boundary_boxes_remain_explicitly_unresolved() {
    let wide_parabola = bezier(vec![
        Point2::new(-1.0, 1.0),
        Point2::new(0.0, -1.0),
        Point2::new(1.0, 1.0),
    ]);
    let vertical = bezier(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);
    let bounded = CertifiedCurveIntersectionOptions::new(1e-10, 100_000, 20).unwrap();

    let outcome = intersect_curve2_certified(&wide_parabola, &vertical, bounded).unwrap();
    assert!(matches!(
        outcome,
        CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Unresolved,
            ref candidate_boxes,
            ..
        } if !candidate_boxes.is_empty()
    ));
}

#[test]
fn certifies_disjoint_planar_curves_without_roots() {
    let first = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![Point2::new(0.0, 1.0), Point2::new(1.0, 1.0)]);

    let outcome = intersect_curve2_certified(&first, &second, options(10_000)).unwrap();
    assert!(matches!(
        outcome,
        CertifiedCurveIntersection2::Complete {
            ref intersections,
            ..
        } if intersections.is_empty()
    ));
}

#[test]
fn zero_length_line_is_classified_by_exact_point_segment_predicates() {
    let off_line = bezier(vec![Point2::new(0.5, 0.6), Point2::new(0.5, 0.6)]);
    let on_line = bezier(vec![Point2::new(0.5, 0.5), Point2::new(0.5, 0.5)]);
    let diagonal = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)]);

    for (first, second) in [(&off_line, &diagonal), (&diagonal, &off_line)] {
        assert!(matches!(
            intersect_curve2_certified(first, second, options(1_024)).unwrap(),
            CertifiedCurveIntersection2::Complete {
                ref intersections,
                ..
            } if intersections.is_empty()
        ));
    }
    for (first, second) in [(&on_line, &diagonal), (&diagonal, &on_line)] {
        assert!(matches!(
            intersect_curve2_certified(first, second, options(1_024)).unwrap(),
            CertifiedCurveIntersection2::Degenerate {
                classification: CurveIntersectionDegeneracy::Tangency,
                ..
            }
        ));
    }
}

#[test]
fn identical_zero_length_curve_is_not_positive_dimensional_overlap() {
    let point = bezier(vec![Point2::new(0.5, 0.5), Point2::new(0.5, 0.5)]);
    assert!(matches!(
        intersect_curve2_certified(&point, &point, options(1_024)).unwrap(),
        CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Tangency,
            ..
        }
    ));
}

#[test]
fn reports_collinear_endpoint_contact_without_overlap() {
    let first = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![Point2::new(1.0, 0.0), Point2::new(2.0, 0.0)]);
    let outcome = intersect_curve2_certified(&first, &second, options(1_024)).unwrap();
    assert!(matches!(
        outcome,
        CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Tangency,
            ..
        }
    ));
}

#[test]
fn reports_structurally_identical_curves_as_overlap() {
    let curve = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)]);
    let outcome = intersect_curve2_certified(&curve, &curve, options(10_000)).unwrap();

    assert!(matches!(
        outcome,
        CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Overlap,
            ..
        }
    ));
}

#[test]
fn identical_multispan_overlap_reports_only_corresponding_parameter_boxes() {
    let curve = BSplineCurve {
        degree: 1,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 0.0),
        ],
        knots: vec![0.0, 1.0, 2.0],
        multiplicities: vec![2, 1, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    };
    let outcome = intersect_curve2_certified(&curve, &curve, options(1_024)).unwrap();
    let CertifiedCurveIntersection2::Degenerate {
        classification: CurveIntersectionDegeneracy::Overlap,
        candidate_boxes,
        ..
    } = outcome
    else {
        panic!("an identical curve must be structurally overlapping");
    };
    assert_eq!(candidate_boxes.len(), 2);
    assert!(candidate_boxes
        .iter()
        .all(|candidate| candidate.first == candidate.second));
}

#[test]
fn reports_a_structural_shared_endpoint_tangency() {
    let first = bezier(vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![
        Point2::new(0.0, 0.0),
        Point2::new(0.5, 0.0),
        Point2::new(1.0, 1.0),
    ]);

    let outcome = intersect_curve2_certified(&first, &second, options(100_000)).unwrap();
    assert!(matches!(
        outcome,
        CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Tangency,
            ..
        }
    ));
}

#[test]
fn root_isolation_fails_closed_on_an_insufficient_budget() {
    let first = bezier(vec![Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0)]);
    let second = bezier(vec![Point2::new(0.0, -1.0), Point2::new(0.0, 1.0)]);
    let error = intersect_curve2_certified(&first, &second, options(1)).unwrap_err();
    assert!(matches!(error, GeomError::BudgetExceeded { .. }));
}
