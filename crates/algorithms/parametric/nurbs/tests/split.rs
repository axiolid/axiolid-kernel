use axiolid_core::Point2;
use axiolid_curve::{BSplineCurve, Curve2, KnotSpec};
use axiolid_nurbs::{bezier_segments2, split2};
use axiolid_scalar::curve::evaluate2;

fn circle() -> BSplineCurve<Point2> {
    let w = 0.5_f64.sqrt();
    BSplineCurve {
        degree: 2,
        control_points: vec![
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(-1.0, 1.0),
            Point2::new(-1.0, 0.0),
            Point2::new(-1.0, -1.0),
            Point2::new(0.0, -1.0),
            Point2::new(1.0, -1.0),
            Point2::new(1.0, 0.0),
        ],
        knots: vec![0.0, 1.0, 2.0, 3.0, 4.0],
        multiplicities: vec![3, 2, 2, 2, 3],
        weights: Some(vec![1.0, w, 1.0, w, 1.0, w, 1.0, w, 1.0]),
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: true,
        self_intersect: Some(false),
    }
}

fn eval(c: &BSplineCurve<Point2>, t: f64) -> Point2 {
    evaluate2(&Curve2::BSpline(c.clone()), t).unwrap()
}

#[test]
fn split_is_exact_and_shares_the_cut_point() {
    let c = circle();
    let (left, right) = split2(&c, 0.5).unwrap();
    assert!(eval(&left, 0.5).distance(eval(&right, 0.5)) < 1e-12);
    for i in 0..=16 {
        let t = 0.5 * f64::from(i) / 16.0;
        assert!(eval(&left, t).distance(eval(&c, t)) < 1e-12);
    }
    for i in 0..=16 {
        let t = 0.5 + 3.5 * f64::from(i) / 16.0;
        assert!(eval(&right, t).distance(eval(&c, t)) < 1e-12);
    }
    assert!(!left.closed && !right.closed);
}

#[test]
fn bezier_decomposition_preserves_the_four_rational_arcs() {
    let segments = bezier_segments2(&circle()).unwrap();
    assert_eq!(segments.len(), 4);
    for segment in &segments {
        assert_eq!(segment.control_points.len(), 3);
        assert_eq!(segment.knots.len(), 2);
        assert_eq!(segment.multiplicities, vec![3, 3]);
        assert!(segment.weights.is_some());
    }
}

#[test]
fn split_rejects_domain_boundaries() {
    let c = circle();
    assert!(split2(&c, 0.0).is_err());
    assert!(split2(&c, 4.0).is_err());
}
