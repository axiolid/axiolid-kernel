use axiolid_core::{Point2, Point3, Vec2, Vec3};
use axiolid_curve::{BSplineCurve, Curve2, Curve3, KnotSpec};
use axiolid_scalar::curve::{jet2, jet3};
use axiolid_scalar::surface::jet;
use axiolid_surface::{BSplineSurface, Surface};

const EPS: f64 = 1.0e-12;

fn assert_vec2(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= EPS,
        "actual {actual:?}, expected {expected:?}"
    );
}

fn assert_vec3(actual: Vec3, expected: Vec3) {
    assert!(
        (actual - expected).length() <= EPS,
        "actual {actual:?}, expected {expected:?}"
    );
}

fn quarter_circle2() -> Curve2 {
    Curve2::BSpline(BSplineCurve {
        degree: 2,
        control_points: vec![
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ],
        knots: vec![0.0, 1.0],
        multiplicities: vec![3, 3],
        weights: Some(vec![1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0]),
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: None,
    })
}

#[test]
fn rational_quarter_circle_has_analytic_second_order_jet() {
    // Independent symbolic oracle for the rational quadratic Bezier at t=1/2:
    // x=y=sqrt(2)/2, x'=-4+2sqrt(2), y'=4-2sqrt(2),
    // x''=y''=32-24sqrt(2).
    let result = jet2(&quarter_circle2(), 0.5).expect("valid rational curve");
    let root2 = 2.0_f64.sqrt();
    assert_vec2(result.point, Point2::splat(root2 / 2.0));
    assert_vec2(
        result.first,
        Vec2::new(-4.0 + 2.0 * root2, 4.0 - 2.0 * root2),
    );
    assert_vec2(result.second, Vec2::splat(32.0 - 24.0 * root2));
}

#[test]
fn rational_quarter_circle_off_midpoint_exercises_weight_derivative_terms() {
    // Independent symbolic quotient-rule oracle at t=1/4, where w' is non-zero.
    let result = jet2(&quarter_circle2(), 0.25).expect("valid rational curve");
    assert_vec2(
        result.point,
        Point2::new(0.929_788_301_062_430_3, 0.368_094_709_561_872_76),
    );
    assert_vec2(
        result.first,
        Vec2::new(-0.584_795_521_488_901_8, 1.477_163_404_606_574),
    );
    assert_vec2(
        result.second,
        Vec2::new(-2.539_200_096_865_833, -0.443_035_386_012_547_3),
    );
}

#[test]
fn degree_one_curve_has_zero_second_derivative() {
    let curve = Curve3::BSpline(BSplineCurve {
        degree: 1,
        control_points: vec![Point3::new(-1.0, 2.0, 4.0), Point3::new(3.0, 6.0, 8.0)],
        knots: vec![2.0, 5.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: None,
    });
    let result = jet3(&curve, 3.25).expect("valid linear B-spline");
    assert_vec3(result.second, Vec3::ZERO);
}

#[test]
fn degree_one_rational_curve_retains_quotient_second_derivative() {
    // x(t) = 2t / (1 + t), so x''(1/2) = -4 / (3/2)^3.
    let curve = Curve2::BSpline(BSplineCurve {
        degree: 1,
        control_points: vec![Point2::ZERO, Point2::X],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: Some(vec![1.0, 2.0]),
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: None,
    });
    let result = jet2(&curve, 0.5).expect("valid rational line");
    assert_vec2(result.second, Vec2::new(-4.0 / 1.5_f64.powi(3), 0.0));
}

fn quarter_cylinder_patch() -> Surface {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    Surface::BSpline(BSplineSurface {
        u_degree: 2,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 2.0)],
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 2.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 2.0)],
        ],
        u_knots: vec![0.0, 1.0],
        v_knots: vec![0.0, 1.0],
        u_multiplicities: vec![3, 3],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![w, w], vec![1.0, 1.0]]),
        knot_spec: KnotSpec::PiecewiseBezier,
        u_closed: false,
        v_closed: false,
        self_intersect: None,
    })
}

#[test]
fn rational_quarter_cylinder_has_all_second_order_partials() {
    let result = jet(&quarter_cylinder_patch(), 0.5, 0.25).expect("valid rational patch");
    let root2 = 2.0_f64.sqrt();
    assert_vec3(result.point, Point3::new(root2 / 2.0, root2 / 2.0, 0.5));
    assert_vec3(
        result.du,
        Vec3::new(-4.0 + 2.0 * root2, 4.0 - 2.0 * root2, 0.0),
    );
    assert_vec3(result.dv, Vec3::new(0.0, 0.0, 2.0));
    assert_vec3(
        result.duu,
        Vec3::new(32.0 - 24.0 * root2, 32.0 - 24.0 * root2, 0.0),
    );
    assert_vec3(result.duv, Vec3::ZERO);
    assert_vec3(result.dvv, Vec3::ZERO);
}
