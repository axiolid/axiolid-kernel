#![cfg(feature = "nurbs")]

use axiolid::curve::{BSplineCurve, Curve2, KnotSpec};
use axiolid::nurbs::{analyze_curve2, insert_knot2};
use axiolid::{Point2, Tolerance};

#[test]
fn facade_exposes_general_nurbs_algorithms() {
    let curve = BSplineCurve {
        degree: 1,
        control_points: vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    };
    let differential = analyze_curve2(
        &Curve2::BSpline(curve.clone()),
        0.5,
        Tolerance::new(1e-12, 1e-12).unwrap(),
    )
    .unwrap();
    assert_eq!(differential.curvature, 0.0);
    assert_eq!(insert_knot2(&curve, 0.5).unwrap().control_points.len(), 3);
}
