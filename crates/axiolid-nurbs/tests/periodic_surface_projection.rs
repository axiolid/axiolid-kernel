use axiolid_core::{Point3, Tolerance};
use axiolid_curve::KnotSpec;
use axiolid_nurbs::{
    project_periodic_surface_certified, CertifiedSurfaceProjection3,
    CertifiedSurfaceProjectionOptions, PeriodicBSplineSurface, SurfaceProjectionCertificate3,
};
use axiolid_surface::BSplineSurface;

fn options(distance: f64, parameter: f64) -> CertifiedSurfaceProjectionOptions {
    CertifiedSurfaceProjectionOptions::new(
        Tolerance::new(distance, 1e-12).unwrap(),
        parameter,
        100_000,
        64,
    )
    .unwrap()
}

fn complete(result: CertifiedSurfaceProjection3) -> SurfaceProjectionCertificate3 {
    match result {
        CertifiedSurfaceProjection3::Complete(certificate) => certificate,
        other => panic!("expected complete periodic certificate: {other:?}"),
    }
}

fn ring() -> [Point3; 5] {
    [
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(-1.0, 0.0, 0.0),
        Point3::new(0.0, -1.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
    ]
}

fn periodic_u_strip() -> PeriodicBSplineSurface {
    let control_points = ring()
        .into_iter()
        .map(|point| vec![point, point + Point3::Z])
        .collect();
    PeriodicBSplineSurface::new(BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points,
        u_knots: vec![-1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        u_multiplicities: vec![1; 7],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: true,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    })
    .unwrap()
}

fn doubly_periodic_surface() -> PeriodicBSplineSurface {
    let u_ring = ring();
    let v_wave = [0.5, 0.0, -0.5, 0.0, 0.5];
    let control_points = u_ring
        .into_iter()
        .map(|u| {
            v_wave
                .into_iter()
                .map(|z| Point3::new(u.x, u.y, z))
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
        weights: None,
        u_closed: true,
        v_closed: true,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    })
    .unwrap()
}

fn covers(certificate: &SurfaceProjectionCertificate3, u: f64, v: f64) -> bool {
    certificate
        .possible_minimizer_boxes
        .iter()
        .any(|cell| cell.u.start <= u && u <= cell.u.end && cell.v.start <= v && v <= cell.v.end)
}

#[test]
fn certifies_periodic_seam_and_nonperiodic_boundary_minima() {
    let surface = periodic_u_strip();
    for (target, expected_distance, expected_v) in [
        (Point3::new(1.5, 0.0, 0.5), 0.5, 0.5),
        (Point3::new(1.5, 0.0, -1.0), 1.0_f64.hypot(0.5), 0.0),
    ] {
        let certificate = complete(
            project_periodic_surface_certified(&surface, target, options(1e-8, 1e-4)).unwrap(),
        );
        assert!(certificate.distance_lower_bound <= expected_distance);
        assert!(certificate.distance_upper_bound >= expected_distance);
        assert!(certificate.gap() <= 1e-8);
        assert!(covers(&certificate, 0.0, expected_v) || covers(&certificate, 4.0, expected_v));
    }
}

#[test]
fn certifies_global_choice_among_multiple_periodic_local_minima() {
    let surface = periodic_u_strip();
    let certificate = complete(
        project_periodic_surface_certified(
            &surface,
            Point3::new(0.1, 0.2, 0.5),
            options(1e-2, 5e-2),
        )
        .unwrap(),
    );
    let expected = 0.7 / 2.0_f64.sqrt();
    assert!(certificate.distance_lower_bound <= expected);
    assert!(certificate.distance_upper_bound >= expected);
    assert!(certificate.gap() <= 1e-2);
    assert!(covers(&certificate, 0.55, 0.5));
}

#[test]
fn doubly_periodic_projection_covers_both_seams_and_matches_dense_reference() {
    let surface = doubly_periodic_surface();
    let target = Point3::new(1.35, -0.08, 0.72);
    let certificate = complete(
        project_periodic_surface_certified(&surface, target, options(1e-7, 5e-4)).unwrap(),
    );

    let mut sampled_minimum = f64::INFINITY;
    for u_index in 0..=200 {
        for v_index in 0..=200 {
            let u = 4.0 * f64::from(u_index) / 200.0;
            let v = 4.0 * f64::from(v_index) / 200.0;
            let point = surface.point(u, v).unwrap();
            sampled_minimum = sampled_minimum.min(point.distance(target));
        }
    }
    assert!(certificate.distance_lower_bound <= sampled_minimum);
    assert!(certificate.distance_upper_bound <= sampled_minimum + 2e-3);
    assert!(certificate.gap() <= 1e-7);
    assert!(certificate.u <= 5e-3 || certificate.u >= 4.0 - 5e-3);
    assert!(certificate.v <= 5e-3 || certificate.v >= 4.0 - 5e-3);
}
