use axiolid_core::{Point2, Tolerance};
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_nurbs::{project_curve2, ProjectionOptions, ProjectionStatus};

fn curve() -> BSplineCurve<Point2> {
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
        self_intersect: None,
    }
}

fn options() -> ProjectionOptions {
    ProjectionOptions::new(Tolerance::new(1e-12, 1e-12).unwrap(), 8, 24, 256).unwrap()
}

#[test]
fn projects_to_rational_quarter_circle_with_bounded_newton() {
    let s = 0.5_f64.sqrt();
    let result = project_curve2(&curve(), Point2::new(2.0 * s, 2.0 * s), options()).unwrap();
    assert!((result.parameter - 0.5).abs() < 1e-10);
    assert!((result.point.x - s).abs() < 1e-10);
    assert!((result.point.y - s).abs() < 1e-10);
    assert!((result.distance - 1.0).abs() < 1e-10);
    assert!(!result.on_boundary);
    assert_eq!(result.status, ProjectionStatus::Converged);
    assert!(result.iterations <= 24);
}

#[test]
fn projection_includes_domain_endpoints() {
    let result = project_curve2(&curve(), Point2::new(2.0, -1.0), options()).unwrap();
    assert_eq!(result.parameter, 0.0);
    assert!(result.point.distance(Point2::new(1.0, 0.0)) < 1e-12);
    assert!(result.on_boundary);
}

#[test]
fn projection_options_reject_unbounded_or_vacuous_work() {
    let tolerance = Tolerance::new(1e-9, 1e-9).unwrap();
    assert!(ProjectionOptions::new(tolerance, 0, 10, 10).is_err());
    assert!(ProjectionOptions::new(tolerance, 4, 0, 10).is_err());
    assert!(ProjectionOptions::new(tolerance, 4, 10, 0).is_err());
}

#[test]
fn projection_fails_closed_when_the_aggregate_start_budget_is_too_small() {
    let policy = ProjectionOptions::new(Tolerance::new(1e-12, 1e-12).unwrap(), 8, 24, 1).unwrap();
    let error = project_curve2(&curve(), Point2::new(2.0, 2.0), policy).unwrap_err();
    assert!(matches!(
        error,
        axiolid_kernel::GeomError::BudgetExceeded { .. }
    ));
}

#[test]
fn projection_rejects_hostile_knot_multiplicity_without_expanding_it() {
    for multiplicity in [1_000_000, u32::MAX] {
        let mut hostile = curve();
        hostile.multiplicities = vec![multiplicity, 3];

        let error = project_curve2(&hostile, Point2::ZERO, options()).unwrap_err();
        assert!(matches!(
            error,
            axiolid_kernel::GeomError::InvalidInput(message)
                if message.contains("compact knot vector")
        ));
    }
}
