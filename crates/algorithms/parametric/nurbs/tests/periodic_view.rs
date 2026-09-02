use axiolid_core::{Point2, Point3, Tolerance};
use axiolid_curve::{BSplineCurve, BSplineCurve3, KnotSpec};
use axiolid_evaluate::curve::bspline_jet2;
use axiolid_nurbs::{
    insert_knot2, reverse2, split2, PeriodicCurve2, PeriodicCurve3, SeamContinuity,
};

fn tolerance() -> Tolerance {
    Tolerance::new(1.0e-12, 1.0e-12).expect("valid tolerance")
}

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

fn circle3() -> BSplineCurve3 {
    let source = circle();
    BSplineCurve {
        degree: source.degree,
        control_points: source
            .control_points
            .into_iter()
            .map(|point| Point3::new(point.x, point.y, 2.0))
            .collect(),
        knots: source.knots,
        multiplicities: source.multiplicities,
        weights: source.weights,
        knot_spec: source.knot_spec,
        closed: source.closed,
        self_intersect: source.self_intersect,
    }
}

fn position_only_seam() -> BSplineCurve<Point2> {
    BSplineCurve {
        degree: 1,
        control_points: vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 0.0),
        ],
        knots: vec![0.0, 0.5, 1.0],
        multiplicities: vec![2, 1, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: true,
        self_intersect: Some(false),
    }
}

#[test]
fn verified_periodic_view_wraps_evaluation_and_jets() {
    let curve = circle();
    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).expect("verified seam");
    assert_eq!(periodic.domain(), (0.0, 4.0));
    assert_eq!(periodic.seam_continuity(), SeamContinuity::FirstDerivative);

    for (outside, native) in [(-0.5, 3.5), (4.25, 0.25), (4_000_000.25, 0.25)] {
        let expected = bspline_jet2(&curve, native).expect("native evaluation");
        let actual = periodic.jet(outside).expect("wrapped evaluation");
        assert!(actual.point.distance(expected.point) < 1.0e-12);
        assert!((actual.first - expected.first).length() < 1.0e-12);
        assert!((periodic.evaluate(outside).unwrap() - expected.point).length() < 1.0e-12);
    }

    assert_eq!(periodic.wrap_parameter(4.0).unwrap(), 4.0);
    assert_eq!(
        periodic.evaluate(4.0).unwrap(),
        bspline_jet2(&curve, 4.0).unwrap().point
    );
    assert!(periodic.evaluate(f64::NAN).is_err());
}

#[test]
fn position_only_seam_is_periodic_but_does_not_claim_derivative_continuity() {
    let curve = position_only_seam();
    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).expect("C0 seam is sufficient");
    assert_eq!(periodic.seam_continuity(), SeamContinuity::Position);
    let lo = periodic.jet(0.0).unwrap();
    let hi = periodic.jet(1.0).unwrap();
    assert_eq!(lo.point, hi.point);
    assert_ne!(lo.first, hi.first);
    assert_eq!(periodic.jet(1.0).unwrap().first, hi.first);
    assert_eq!(periodic.jet(2.0).unwrap().first, lo.first);
}

#[test]
fn spatial_periodic_view_evaluates_and_edits_the_same_cycle() {
    let curve = circle3();
    let periodic = PeriodicCurve3::try_new(&curve, tolerance()).unwrap();
    let wrapped = periodic.evaluate(4_000_000_000.25).unwrap();
    let native = periodic.evaluate(0.25).unwrap();
    assert!(wrapped.distance(native) < 1e-12);
    assert_eq!(wrapped.z, 2.0);

    let edited = periodic.insert_knot(-3.5).unwrap();
    let edited_view = PeriodicCurve3::try_new(&edited, tolerance()).unwrap();
    assert!(edited_view.evaluate(4.25).unwrap().distance(native) < 1e-12);
}

#[test]
fn periodic_wrapping_rejects_non_finite_inputs() {
    let curve = circle();
    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).unwrap();
    for parameter in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(periodic.wrap_parameter(parameter).is_err());
        assert!(periodic.evaluate(parameter).is_err());
        assert!(periodic.insert_knot(parameter).is_err());
        assert!(periodic.split_at(parameter).is_err());
    }
}

#[test]
fn neutral_evaluator_does_not_wrap_and_neutral_editing_remains_strict() {
    let curve = circle();
    let neutral = bspline_jet2(&curve, 4.25).unwrap().point;
    assert!(insert_knot2(&curve, 4.5).is_err());
    assert!(split2(&curve, -3.5).is_err());

    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).unwrap();
    let wrapped = periodic.evaluate(4.25).unwrap();
    assert!(neutral.distance(wrapped) > 1e-3);
    assert!(periodic.insert_knot(4.5).is_ok());
    assert!(periodic.split_at(-3.5).is_ok());
}

#[test]
fn flags_or_geometry_alone_cannot_create_a_periodic_view() {
    let mut open_flag = circle();
    open_flag.closed = false;
    assert!(PeriodicCurve2::try_new(&open_flag, tolerance()).is_err());

    let mut open_seam = circle();
    open_seam.control_points[8] = Point2::new(2.0, 0.0);
    assert!(PeriodicCurve2::try_new(&open_seam, tolerance()).is_err());
}

#[test]
fn periodic_insertion_wraps_to_native_interior_and_preserves_the_seam() {
    let curve = circle();
    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).unwrap();
    let edited = periodic
        .insert_knot(4.5)
        .expect("exterior parameter wraps to 0.5");
    assert!(edited.knots.contains(&0.5));
    assert!(edited
        .weights
        .as_ref()
        .unwrap()
        .iter()
        .all(|weight| *weight > 0.0));
    let edited_periodic = PeriodicCurve2::try_new(&edited, tolerance()).expect("seam preserved");
    assert_eq!(
        edited_periodic.seam_continuity(),
        SeamContinuity::FirstDerivative
    );

    for sample in 0..=32 {
        let parameter = 4.0 * f64::from(sample) / 32.0;
        let before = bspline_jet2(&curve, parameter).unwrap().point;
        let after = bspline_jet2(&edited, parameter).unwrap().point;
        assert!(before.distance(after) < 1.0e-12);
    }

    assert!(periodic.insert_knot(4.0).is_err());
    assert!(periodic.insert_knot(8.0).is_err());
}

#[test]
fn periodic_split_canonicalizes_the_cut_and_returns_open_exact_pieces() {
    let curve = circle();
    let periodic = PeriodicCurve2::try_new(&curve, tolerance()).unwrap();
    let (left, right) = periodic.split_at(4.5).expect("cut wraps to 0.5");
    assert!(!left.closed);
    assert!(!right.closed);
    let expected = bspline_jet2(&curve, 0.5).unwrap().point;
    assert!(bspline_jet2(&left, 0.5).unwrap().point.distance(expected) < 1.0e-12);
    assert!(bspline_jet2(&right, 0.5).unwrap().point.distance(expected) < 1.0e-12);
    assert!(periodic.split_at(4.0).is_err());
}

#[test]
fn reversal_preserves_verified_periodic_semantics() {
    let reversed = reverse2(&circle()).expect("exact reversal");
    let periodic = PeriodicCurve2::try_new(&reversed, tolerance()).expect("seam retained");
    assert_eq!(periodic.seam_continuity(), SeamContinuity::FirstDerivative);
    assert!(periodic.evaluate(-0.25).is_ok());
}
