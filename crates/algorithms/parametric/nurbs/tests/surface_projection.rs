use axiolid_core::{Point3, Tolerance};
use axiolid_curve::KnotSpec;
use axiolid_nurbs::{project_surface, ProjectionOptions, ProjectionStatus};
use axiolid_surface::BSplineSurface;

fn surface() -> BSplineSurface {
    let w = 0.5_f64.sqrt();
    BSplineSurface {
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
    }
}

#[test]
fn projects_to_rational_quarter_cylinder_in_both_parameters() {
    let s = 0.5_f64.sqrt();
    let result = project_surface(
        &surface(),
        Point3::new(2.0 * s, 2.0 * s, 0.5),
        ProjectionOptions::new(Tolerance::new(1e-11, 1e-11).unwrap(), 5, 24, 256).unwrap(),
    )
    .unwrap();
    assert!((result.u - 0.5).abs() < 1e-9);
    assert!((result.v - 0.25).abs() < 1e-9);
    assert!(result.point.distance(Point3::new(s, s, 0.5)) < 1e-9);
    assert!((result.distance - 1.0).abs() < 1e-9);
    assert_eq!(result.status, ProjectionStatus::Converged);
    assert!(result.iterations <= 24);
}
