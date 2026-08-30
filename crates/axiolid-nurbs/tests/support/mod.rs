use axiolid_core::Point3;
use axiolid_curve::KnotSpec;
use axiolid_surface::BSplineSurface;

pub fn quarter_cylinder() -> BSplineSurface {
    let w = 2.0_f64.sqrt() / 2.0;
    BSplineSurface {
        u_degree: 2,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 2.0)],
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 2.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 2.0)],
        ],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![3, 3],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![w, w], vec![1.0, 1.0]]),
        knot_spec: KnotSpec::PiecewiseBezier,
        u_closed: false,
        v_closed: false,
        self_intersect: Some(false),
    }
}
