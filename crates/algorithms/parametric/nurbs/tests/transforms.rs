use axiolid_core::{Point2, Point3};
use axiolid_curve::{BSplineCurve, Curve2, Curve3, KnotSpec};
use axiolid_nurbs::{insert_knot2, insert_knot3, reverse2, reverse3};
use axiolid_scalar::{evaluate2, evaluate3};

fn quarter_circle2() -> BSplineCurve<Point2> {
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

fn quarter_circle3() -> BSplineCurve<Point3> {
    let c = quarter_circle2();
    BSplineCurve {
        degree: c.degree,
        control_points: c
            .control_points
            .into_iter()
            .map(|p| Point3::new(p.x, p.y, 2.0))
            .collect(),
        knots: c.knots,
        multiplicities: c.multiplicities,
        weights: c.weights,
        knot_spec: c.knot_spec,
        closed: c.closed,
        self_intersect: c.self_intersect,
    }
}

#[test]
fn knot_insertion_preserves_rational_curve_shape_in_2d_and_3d() {
    let original2 = quarter_circle2();
    let inserted2 = insert_knot2(&original2, 0.5).expect("insert interior knot");
    assert_eq!(inserted2.control_points.len(), 4);
    assert_eq!(inserted2.knots, vec![0.0, 0.5, 1.0]);
    assert_eq!(inserted2.multiplicities, vec![3, 1, 3]);

    let original3 = quarter_circle3();
    let inserted3 = insert_knot3(&original3, 0.5).expect("insert interior knot");
    for i in 0..=32 {
        let t = f64::from(i) / 32.0;
        assert!(
            evaluate2(&Curve2::BSpline(original2.clone()), t)
                .unwrap()
                .distance(evaluate2(&Curve2::BSpline(inserted2.clone()), t).unwrap())
                < 1e-12
        );
        assert!(
            evaluate3(&Curve3::BSpline(original3.clone()), t)
                .unwrap()
                .distance(evaluate3(&Curve3::BSpline(inserted3.clone()), t).unwrap())
                < 1e-12
        );
    }
}

#[test]
fn reversal_preserves_shape_with_reversed_parameter() {
    let original2 = quarter_circle2();
    let reversed2 = reverse2(&original2).expect("reverse valid curve");
    let original3 = quarter_circle3();
    let reversed3 = reverse3(&original3).expect("reverse valid curve");
    for i in 0..=32 {
        let t = f64::from(i) / 32.0;
        assert!(
            evaluate2(&Curve2::BSpline(original2.clone()), t)
                .unwrap()
                .distance(evaluate2(&Curve2::BSpline(reversed2.clone()), 1.0 - t).unwrap())
                < 1e-12
        );
        assert!(
            evaluate3(&Curve3::BSpline(original3.clone()), t)
                .unwrap()
                .distance(evaluate3(&Curve3::BSpline(reversed3.clone()), 1.0 - t).unwrap())
                < 1e-12
        );
    }
}

#[test]
fn insertion_rejects_endpoints_and_full_multiplicity() {
    let curve = quarter_circle2();
    assert!(insert_knot2(&curve, 0.0).is_err());
    let once = insert_knot2(&curve, 0.5).unwrap();
    let twice = insert_knot2(&once, 0.5).unwrap();
    assert!(insert_knot2(&twice, 0.5).is_err());
}

#[test]
fn reversal_rejects_a_non_finite_reflected_knot_origin() {
    let mut curve = quarter_circle2();
    curve.knots = vec![0.75 * f64::MAX, f64::MAX];
    assert!(reverse2(&curve).is_err());
}
