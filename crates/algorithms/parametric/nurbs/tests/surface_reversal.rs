mod support;

use axiolid_evaluate::surface::bspline_jet;
use axiolid_nurbs::{reverse_surface_u, reverse_surface_v};
use support::quarter_cylinder;

#[test]
fn axis_reversal_preserves_surface_with_reversed_parameter() {
    let surface = quarter_cylinder();
    let u_reversed = reverse_surface_u(&surface).unwrap();
    let v_reversed = reverse_surface_v(&surface).unwrap();
    for i in 0..=10 {
        for j in 0..=10 {
            let u = f64::from(i) / 10.0;
            let v = f64::from(j) / 10.0;
            let expected = bspline_jet(&surface, u, v).unwrap().point;
            assert!(
                bspline_jet(&u_reversed, 1.0 - u, v)
                    .unwrap()
                    .point
                    .distance(expected)
                    < 1e-12
            );
            assert!(
                bspline_jet(&v_reversed, u, 1.0 - v)
                    .unwrap()
                    .point
                    .distance(expected)
                    < 1e-12
            );
        }
    }
}

#[test]
fn surface_reversal_rejects_a_non_finite_reflected_knot_origin() {
    let mut surface = quarter_cylinder();
    surface.u_knots = vec![0.75 * f64::MAX, f64::MAX];

    let error = reverse_surface_u(&surface).unwrap_err();
    assert!(error.to_string().contains("finite"));
}
