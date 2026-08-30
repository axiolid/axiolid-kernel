use axiolid_core::{Point2, Tolerance};
use axiolid_curve::{BSplineCurve, KnotSpec};
use axiolid_nurbs::{curve2_seam_continuity, wrap_curve2_parameter, SeamContinuity};

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

#[test]
fn closed_rational_circle_has_first_derivative_seam() {
    assert_eq!(
        curve2_seam_continuity(&circle(), Tolerance::new(1e-12, 1e-12).unwrap()).unwrap(),
        SeamContinuity::FirstDerivative
    );
}

#[test]
fn verified_closed_curve_wraps_outside_parameters() {
    let c = circle();
    let tolerance = Tolerance::new(1e-12, 1e-12).unwrap();
    assert!((wrap_curve2_parameter(&c, -0.5, tolerance).unwrap() - 3.5).abs() < 1e-12);
    assert!((wrap_curve2_parameter(&c, 4.25, tolerance).unwrap() - 0.25).abs() < 1e-12);
    assert_eq!(wrap_curve2_parameter(&c, 4.0, tolerance).unwrap(), 4.0);
}

#[test]
fn closed_metadata_cannot_enable_wrapping_when_seam_is_open() {
    let mut c = circle();
    c.control_points[8] = Point2::new(2.0, 0.0);
    let tolerance = Tolerance::new(1e-12, 1e-12).unwrap();
    assert_eq!(
        curve2_seam_continuity(&c, tolerance).unwrap(),
        SeamContinuity::Discontinuous
    );
    assert!(wrap_curve2_parameter(&c, -0.5, tolerance).is_err());
}

#[test]
fn an_open_declaration_cannot_enable_wrapping_when_the_seam_is_geometric() {
    let mut c = circle();
    c.closed = false;
    assert!(wrap_curve2_parameter(&c, -0.5, Tolerance::new(1e-12, 1e-12).unwrap()).is_err());
}
