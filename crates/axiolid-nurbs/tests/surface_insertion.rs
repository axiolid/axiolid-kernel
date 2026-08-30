mod support;

use axiolid_nurbs::{insert_surface_knot_u, insert_surface_knot_v};
use axiolid_scalar::surface::bspline_jet;
use support::quarter_cylinder;

#[test]
fn axis_knot_insertion_preserves_rational_surface() {
    let surface = quarter_cylinder();
    let u_inserted = insert_surface_knot_u(&surface, 0.5).unwrap();
    let v_inserted = insert_surface_knot_v(&surface, 0.25).unwrap();
    assert_eq!(u_inserted.control_points.len(), 4);
    assert_eq!(u_inserted.u_multiplicities, vec![3, 1, 3]);
    assert_eq!(u_inserted.control_points[0].len(), 2);
    assert_eq!(v_inserted.control_points[0].len(), 3);
    assert_eq!(v_inserted.v_multiplicities, vec![2, 1, 2]);
    for i in 0..=10 {
        for j in 0..=10 {
            let u = f64::from(i) / 10.0;
            let v = f64::from(j) / 10.0;
            let expected = bspline_jet(&surface, u, v).unwrap().point;
            assert!(
                bspline_jet(&u_inserted, u, v)
                    .unwrap()
                    .point
                    .distance(expected)
                    < 1e-12
            );
            assert!(
                bspline_jet(&v_inserted, u, v)
                    .unwrap()
                    .point
                    .distance(expected)
                    < 1e-12
            );
        }
    }
}
