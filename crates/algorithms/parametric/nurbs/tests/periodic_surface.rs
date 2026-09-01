use axiolid_core::{Point3, Scalar};
use axiolid_curve::KnotSpec;
use axiolid_nurbs::PeriodicBSplineSurface;
use axiolid_surface::BSplineSurface;

fn periodic_u_strip() -> BSplineSurface {
    let ring = [
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(-1.0, 0.0, 0.0),
        Point3::new(0.0, -1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ];
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: ring
            .into_iter()
            .map(|point| vec![point, Point3::new(point.x, point.y, point.z + 1.0)])
            .collect(),
        u_knots: vec![-1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        u_multiplicities: vec![1; 7],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: true,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    }
}

fn assert_point_near(actual: Point3, expected: Point3, tolerance: Scalar) {
    assert!((actual.x - expected.x).abs() <= tolerance);
    assert!((actual.y - expected.y).abs() <= tolerance);
    assert!((actual.z - expected.z).abs() <= tolerance);
}

#[test]
fn validates_canonical_cyclic_axis_and_wraps_to_half_open_period() {
    let periodic = PeriodicBSplineSurface::new(periodic_u_strip()).expect("canonical periodic U");
    assert_eq!(periodic.u_domain(), (0.0, 4.0));
    assert_eq!(periodic.v_domain(), (0.0, 1.0));
    assert_eq!(periodic.u_seam_continuity_order(), Some(0));
    assert_eq!(periodic.v_seam_continuity_order(), None);
    assert_eq!(periodic.unique_u_control_count(), 4);
    assert_eq!(periodic.unique_v_control_count(), 2);

    let native = periodic.point(0.25, 0.4).expect("native point");
    for equivalent in [-7.75, 4.25, 8.25] {
        assert_point_near(
            periodic.point(equivalent, 0.4).expect("wrapped point"),
            native,
            1e-12,
        );
    }
    assert_point_near(
        periodic.point(4.0, 0.4).expect("upper seam wraps"),
        periodic.point(0.0, 0.4).expect("lower seam"),
        1e-12,
    );
}

#[test]
fn rejects_periodic_offsets_that_cannot_resolve_one_period() {
    let periodic = PeriodicBSplineSurface::new(periodic_u_strip()).expect("periodic U");
    assert_eq!(
        periodic
            .wrap_parameters(9_007_199_254_740_992.0, 0.5)
            .expect("binary64 spacing remains smaller than the period"),
        (0.0, 0.5)
    );
    assert!(periodic.wrap_parameters(1e308, 0.5).is_err());
    assert!(periodic.wrap_parameters(-1e308, 0.5).is_err());
}

#[test]
fn accepts_inexact_domain_width_on_a_nonperiodic_axis() {
    let mut surface = periodic_u_strip();
    surface.v_knots = vec![-1e300, 1.0];

    let periodic = PeriodicBSplineSurface::new(surface)
        .expect("only periodic axes require exact period arithmetic");
    assert_eq!(periodic.v_domain(), (-1e300, 1.0));
}

#[test]
fn rejects_declared_periodicity_without_exact_control_aliases() {
    let mut surface = periodic_u_strip();
    surface.control_points[4][0] = Point3::new(2.0, 0.0, 0.0);
    assert!(PeriodicBSplineSurface::new(surface).is_err());
}

#[test]
fn rejects_declared_periodicity_without_periodic_knot_extension() {
    let mut surface = periodic_u_strip();
    surface.u_knots[6] = 5.5;
    assert!(PeriodicBSplineSurface::new(surface).is_err());
}

fn doubly_periodic_rational_surface() -> PeriodicBSplineSurface {
    let ring = [
        Point3::new(1.0, 0.0, 0.5),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(-1.0, 0.0, -0.5),
        Point3::new(0.0, -1.0, 0.0),
        Point3::new(1.0, 0.0, 0.5),
    ];
    let control_points = ring
        .into_iter()
        .map(|u| {
            ring.into_iter()
                .map(|v| Point3::new(u.x, u.y, v.z))
                .collect()
        })
        .collect();
    PeriodicBSplineSurface::new(BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points,
        u_knots: vec![-1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        u_multiplicities: vec![1; 7],
        v_knots: vec![-1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        v_multiplicities: vec![1; 7],
        weights: Some(vec![vec![1.0; 5]; 5]),
        u_closed: true,
        v_closed: true,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    })
    .unwrap()
}

#[test]
fn wrapped_edits_update_all_doubly_periodic_corner_aliases() {
    let mut surface = doubly_periodic_rational_surface();
    let replacement = Point3::new(2.0, 3.0, 4.0);
    surface
        .set_control_point_wrapped(-4, 4, replacement)
        .unwrap();
    surface.set_weight_wrapped(4, -4, 2.5).unwrap();

    let expanded = surface.as_bspline_surface();
    let weights = expanded.weights.as_ref().unwrap();
    for (u, v) in [(0, 0), (0, 4), (4, 0), (4, 4)] {
        assert_eq!(expanded.control_points[u][v], replacement);
        assert_eq!(weights[u][v], 2.5);
    }
    assert_point_near(
        surface.point(0.25, 0.75).unwrap(),
        surface.point(4.25, -3.25).unwrap(),
        1e-12,
    );
}

#[test]
fn seam_control_edit_updates_every_expanded_alias() {
    let mut periodic = PeriodicBSplineSurface::new(periodic_u_strip()).expect("periodic U");
    let replacement = Point3::new(1.25, 0.0, -0.5);
    periodic
        .set_control_point(0, 0, replacement)
        .expect("unique seam edit");

    let expanded = periodic.as_bspline_surface();
    assert_eq!(expanded.control_points[0][0], replacement);
    assert_eq!(expanded.control_points[4][0], replacement);
    assert_eq!(periodic.unique_u_control_count(), 4);
    assert!(periodic.set_control_point(4, 0, replacement).is_err());
}

#[test]
fn neutral_surface_evaluation_remains_native_and_unwrapped() {
    let surface = periodic_u_strip();
    let neutral_high = axiolid_reference::surface::bspline_jet(&surface, 4.25, 0.4)
        .expect("neutral evaluator clamps")
        .point;
    let periodic = PeriodicBSplineSurface::new(surface).expect("periodic U");
    let wrapped = periodic.point(4.25, 0.4).expect("wrapped evaluator");
    assert_ne!(neutral_high, wrapped);
}
