use axiolid_contracts::GeomError;
use axiolid_core::{Point2, Point3, Tolerance};
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_nurbs::{
    distance_curve2_certified, distance_curve3_certified, CertifiedProjectionOptions,
};

fn line2(start: Point2, end: Point2) -> BSplineCurve<Point2> {
    BSplineCurve {
        degree: 1,
        control_points: vec![start, end],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed: false,
        self_intersect: Some(false),
        knot_spec: KnotSpec::PiecewiseBezier,
    }
}

fn line3(start: Point3, end: Point3) -> BSplineCurve<Point3> {
    BSplineCurve {
        degree: 1,
        control_points: vec![start, end],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed: false,
        self_intersect: Some(false),
        knot_spec: KnotSpec::PiecewiseBezier,
    }
}

fn options(linear: f64, max_nodes: u32) -> CertifiedProjectionOptions {
    CertifiedProjectionOptions::new(Tolerance::new(linear, 1e-12).unwrap(), max_nodes, 64).unwrap()
}

#[test]
fn certifies_the_global_crossing_of_two_planar_curves() {
    let first = line2(Point2::new(-1.0, 0.0), Point2::new(1.0, 0.0));
    let second = line2(Point2::new(0.0, -1.0), Point2::new(0.0, 1.0));
    let result = distance_curve2_certified(&first, &second, options(1e-12, 10_000)).unwrap();

    assert!(result.distance_lower_bound <= 1e-14);
    assert!(result.distance_upper_bound <= 1e-12);
    assert!(result.gap() <= 1e-12);
    assert!((result.first_parameter - 0.5).abs() < 1e-12);
    assert!((result.second_parameter - 0.5).abs() < 1e-12);
    assert!(!result.possible_minimizer_boxes.is_empty());
}

#[test]
fn certifies_the_distance_between_spatially_skew_segments() {
    let first = line3(Point3::new(-1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0));
    let second = line3(Point3::new(0.0, -1.0, 1.0), Point3::new(0.0, 1.0, 1.0));
    let result = distance_curve3_certified(&first, &second, options(1e-12, 10_000)).unwrap();

    assert!(result.distance_lower_bound <= 1.0);
    assert!(result.distance_upper_bound >= 1.0);
    assert!(result.gap() <= 1e-12);
    assert!((result.distance - 1.0).abs() < 1e-12);
}

#[test]
fn pair_certification_fails_closed_before_a_cartesian_root_allocation() {
    let mut first = line2(Point2::ZERO, Point2::X);
    first.control_points = (0..101)
        .map(|index| Point2::new(index as f64, 0.0))
        .collect();
    first.knots = (0..=100).map(|index| index as f64).collect();
    first.multiplicities = vec![1; 101];
    first.multiplicities[0] = 2;
    first.multiplicities[100] = 2;

    let second = line2(Point2::ZERO, Point2::Y);
    let error = distance_curve2_certified(&first, &second, options(1e-9, 10)).unwrap_err();
    assert!(matches!(error, GeomError::BudgetExceeded { .. }));
}
