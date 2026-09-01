use axiolid_core::{Point2, Point3, Tolerance};
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_kernel::GeomError;
use axiolid_nurbs::{
    project_curve2_certified, project_curve3_certified, CertifiedProjectionOptions,
};

fn quarter_circle() -> BSplineCurve<Point2> {
    BSplineCurve {
        degree: 2,
        control_points: vec![
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![3, 3],
        weights: Some(vec![1.0, 0.5_f64.sqrt(), 1.0]),
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    }
}

fn line3() -> BSplineCurve<Point3> {
    BSplineCurve {
        degree: 1,
        control_points: vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    }
}

fn smooth_two_span_curve() -> BSplineCurve<Point2> {
    BSplineCurve {
        degree: 2,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, -1.0),
            Point2::new(3.0, 0.0),
        ],
        knots: vec![0.0, 0.25, 1.0],
        multiplicities: vec![3, 1, 3],
        weights: Some(vec![1.0, 0.75, 1.25, 1.0]),
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: Some(false),
    }
}

fn options(linear: f64, max_nodes: u32) -> CertifiedProjectionOptions {
    CertifiedProjectionOptions::new(Tolerance::new(linear, 1e-12).unwrap(), max_nodes, 64).unwrap()
}

#[test]
fn globally_certifies_the_rational_quarter_circle_minimum() {
    let s = 0.5_f64.sqrt();
    let result = project_curve2_certified(
        &quarter_circle(),
        Point2::new(2.0 * s, 2.0 * s),
        options(1e-10, 1_000_000),
    )
    .unwrap();

    assert!(result.distance_lower_bound <= 1.0);
    assert!(result.distance_upper_bound >= 1.0);
    assert!(result.gap() <= 1e-10);
    assert!((result.parameter - 0.5).abs() < 1e-10);
    assert!((result.point.x - s).abs() < 1e-10);
    assert!((result.point.y - s).abs() < 1e-10);
    assert!((result.distance - 1.0).abs() < 1e-10);
    assert!(result.visited_nodes > 0);
    assert!(!result.possible_minimizer_intervals.is_empty());
}

#[test]
fn globally_certifies_a_spatial_line_projection() {
    let result = project_curve3_certified(
        &line3(),
        Point3::new(0.25, 2.0, 0.0),
        options(1e-12, 10_000),
    )
    .unwrap();

    assert!(result.distance_lower_bound <= 2.0);
    assert!(result.distance_upper_bound >= 2.0);
    assert!(result.gap() <= 1e-12);
    assert!((result.parameter - 0.25).abs() < 1e-12);
    assert!((result.point.x - 0.25).abs() < 1e-12);
    assert!((result.distance - 2.0).abs() < 1e-12);
}

#[test]
fn interval_refinement_certifies_a_smooth_rational_multispan_curve() {
    let result = project_curve2_certified(
        &smooth_two_span_curve(),
        Point2::new(19.0 / 14.0, 2.0 / 7.0),
        options(1e-10, 1_000_000),
    )
    .unwrap();

    assert!(result.distance_lower_bound <= 1e-14);
    assert!(result.distance_upper_bound <= 1e-10);
    assert!(result.gap() <= 1e-10);
    assert!((result.parameter - 0.25).abs() < 1e-10);
}

#[test]
fn certification_fails_closed_when_the_node_budget_is_insufficient() {
    let s = 0.5_f64.sqrt();
    let error = project_curve2_certified(
        &quarter_circle(),
        Point2::new(2.0 * s, 2.0 * s),
        options(1e-12, 1),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GeomError::BudgetExceeded {
            resource: "certified projection nodes"
        }
    ));
}

#[test]
fn certification_rejects_non_positive_rational_weights() {
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut curve = quarter_circle();
        curve.weights.as_mut().unwrap()[1] = weight;
        let error = project_curve2_certified(&curve, Point2::ZERO, options(1e-9, 100)).unwrap_err();
        assert!(matches!(error, GeomError::InvalidInput(_)));
    }
}

#[test]
fn certification_options_reject_vacuous_budgets() {
    let tolerance = Tolerance::new(1e-9, 1e-12).unwrap();
    assert!(CertifiedProjectionOptions::new(tolerance, 0, 32).is_err());
    assert!(CertifiedProjectionOptions::new(tolerance, 100, 0).is_err());
}
