use axiolid_core::{Frame3, Point2, Point3, Tolerance};
use axiolid_curve::{BSplineCurve, Circle3, Curve2, Curve3, KnotSpec, Polyline2};
use axiolid_nurbs::{analyze_curve2, analyze_curve3, analyze_surface};
use axiolid_surface::{Sphere, Surface};

fn quarter_circle() -> Curve2 {
    Curve2::BSpline(BSplineCurve {
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
    })
}

fn world() -> Frame3 {
    Frame3 {
        origin: Point3::ZERO,
        x: Point3::X,
        y: Point3::Y,
        z: Point3::Z,
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}

#[test]
fn rational_quarter_circle_has_unit_parameter_invariant_curvature() {
    let differential = analyze_curve2(
        &quarter_circle(),
        0.5,
        Tolerance::new(1e-12, 1e-12).unwrap(),
    )
    .expect("regular quarter circle");
    close(differential.curvature, 1.0);
    close(differential.signed_curvature, 1.0);
    close(differential.unit_tangent.length(), 1.0);
}

#[test]
fn spatial_circle_has_expected_curvature_vector() {
    let curve = Curve3::Circle(Circle3 {
        frame: world(),
        radius: 2.0,
    });
    let differential =
        analyze_curve3(&curve, 0.0, Tolerance::new(1e-12, 1e-12).unwrap()).expect("regular circle");
    close(differential.curvature, 0.5);
    close(differential.curvature_vector.x, -0.5);
    close(differential.curvature_vector.y, 0.0);
    close(differential.unit_tangent.y, 1.0);
}

#[test]
fn sphere_has_first_and_second_fundamental_form() {
    let surface = Surface::Sphere(Sphere {
        frame: world(),
        radius: 2.0,
    });
    let differential = analyze_surface(&surface, 0.0, 0.0, Tolerance::new(1e-12, 1e-12).unwrap())
        .expect("regular sphere point");
    close(differential.point.distance(Point3::new(2.0, 0.0, 0.0)), 0.0);
    close(differential.first.e, 4.0);
    close(differential.first.f, 0.0);
    close(differential.first.g, 4.0);
    close(differential.gaussian_curvature, 0.25);
    close(differential.mean_curvature, -0.5);
    close(differential.principal_curvatures[0], -0.5);
    close(differential.principal_curvatures[1], -0.5);
}

#[test]
fn polyline_curvature_is_rejected_at_a_nonsmooth_vertex() {
    let curve = Curve2::Polyline(Polyline2 {
        points: vec![Point2::ZERO, Point2::X, Point2::new(1.0, 1.0)],
        closed: false,
    });
    let tolerance = Tolerance::new(1e-12, 1e-12).unwrap();
    assert_eq!(
        analyze_curve2(&curve, 0.5, tolerance).unwrap().curvature,
        0.0
    );
    assert!(analyze_curve2(&curve, 1.0, tolerance).is_err());
}
