use axiolid_core::{Point2, Tolerance};
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_kernel::GeomError;
use axiolid_nurbs::{
    intersect_curve2_certified, CertifiedCurveIntersection2, CertifiedProjectionOptions,
    CurveIntersectionDegeneracy,
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

fn options(max_nodes: u32) -> CertifiedProjectionOptions {
    CertifiedProjectionOptions::new(Tolerance::new(1e-10, 1e-12).unwrap(), max_nodes, 64).unwrap()
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
        panic!("a regular nonlinear crossing must be isolated");
    };
    assert_eq!(intersections.len(), 1);
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
    let bounded =
        CertifiedProjectionOptions::new(Tolerance::new(1e-10, 1e-12).unwrap(), 100_000, 20)
            .unwrap();

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
