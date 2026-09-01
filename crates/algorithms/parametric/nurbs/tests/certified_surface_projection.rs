use axiolid_core::{Point3, Tolerance};
use axiolid_curve::KnotSpec;
use axiolid_kernel::GeomError;
use axiolid_nurbs::{
    project_surface_certified, CertifiedSurfaceProjection3, CertifiedSurfaceProjectionOptions,
    SurfaceProjectionCertificate3, SurfaceProjectionUnresolvedReason,
    MAX_CERTIFIED_SURFACE_PROJECTION_DEPTH, MAX_CERTIFIED_SURFACE_PROJECTION_WORK,
};
use axiolid_reference::surface::bspline_jet;
use axiolid_surface::BSplineSurface;

fn plane() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
            vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![2.0, 5.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![-3.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    }
}

fn multispan_plane() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
            vec![Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![2.0, 3.5, 5.0],
        u_multiplicities: vec![2, 1, 2],
        v_knots: vec![-3.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    }
}

fn options(
    distance: f64,
    parameter: f64,
    nodes: u32,
    depth: u16,
) -> CertifiedSurfaceProjectionOptions {
    CertifiedSurfaceProjectionOptions::new(
        Tolerance::new(distance, 1e-12).unwrap(),
        parameter,
        nodes,
        depth,
    )
    .unwrap()
}

fn complete(result: CertifiedSurfaceProjection3) -> SurfaceProjectionCertificate3 {
    match result {
        CertifiedSurfaceProjection3::Complete(certificate) => certificate,
        other => panic!("expected complete certificate: {other:?}"),
    }
}

fn contains(certificate: &SurfaceProjectionCertificate3, u: f64, v: f64) -> bool {
    certificate
        .possible_minimizer_boxes
        .iter()
        .any(|candidate| {
            candidate.u.start <= u
                && u <= candidate.u.end
                && candidate.v.start <= v
                && v <= candidate.v.end
        })
}

#[test]
fn certifies_affine_native_interior_edge_and_corner_minima() {
    for (target, expected) in [
        (Point3::new(0.0, 0.0, 2.0), (3.5, -1.0, 2.0)),
        (Point3::new(-2.0, 0.0, 3.0), (2.0, -1.0, 1.0_f64.hypot(3.0))),
        (
            Point3::new(-2.0, -4.0, 1.0),
            (2.0, -3.0, 3.0_f64.hypot(1.0_f64.hypot(1.0))),
        ),
    ] {
        let certificate = complete(
            project_surface_certified(&plane(), target, options(1e-8, 1e-5, 100_000, 64)).unwrap(),
        );
        assert!(certificate.distance_lower_bound <= expected.2);
        assert!(certificate.distance_upper_bound >= expected.2);
        assert!(certificate.gap() <= 1e-8);
        assert!((certificate.u - expected.0).abs() <= 2e-5);
        assert!((certificate.v - expected.1).abs() <= 2e-5);
        assert!(contains(&certificate, expected.0, expected.1));
        assert!(certificate
            .possible_minimizer_boxes
            .iter()
            .all(|candidate| {
                candidate.u.end - candidate.u.start <= 1e-5
                    && candidate.v.end - candidate.v.start <= 1e-5
            }));
        let scalar = bspline_jet(&plane(), certificate.u, certificate.v)
            .unwrap()
            .point;
        assert!(scalar.distance(certificate.point) <= 1e-14);
    }
}

#[test]
fn certifies_positive_rational_and_multispan_surfaces() {
    let mut rational = plane();
    rational.weights = Some(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let rational_target = bspline_jet(&rational, 3.5, -1.0).unwrap().point;
    let rational_certificate = complete(
        project_surface_certified(&rational, rational_target, options(1e-8, 1e-4, 100_000, 64))
            .unwrap(),
    );
    assert!(rational_certificate.distance_lower_bound <= 1e-14);
    assert!(rational_certificate.distance_upper_bound <= 1e-8);
    assert!(rational_certificate.distance_upper_bound >= rational_certificate.distance);
    assert!(
        rational_certificate.distance_upper_bound > rational_certificate.distance
            || rational_certificate.distance == 0.0
    );
    assert!(contains(&rational_certificate, 3.5, -1.0));

    let surface = multispan_plane();
    let native = (4.1, -0.25);
    let on_surface = bspline_jet(&surface, native.0, native.1).unwrap().point;
    let target = on_surface + Point3::Z * 2.0;
    let certificate = complete(
        project_surface_certified(&surface, target, options(1e-8, 1e-4, 100_000, 64)).unwrap(),
    );
    assert!(certificate.distance_lower_bound <= 2.0);
    assert!(certificate.distance_upper_bound >= 2.0);
    assert!(certificate.distance_upper_bound > certificate.distance);
    assert!(certificate.gap() <= 1e-8);
    assert!(contains(&certificate, native.0, native.1));
}

#[test]
fn retains_a_continuum_of_global_minimizers_without_claiming_uniqueness() {
    let mut constant = plane();
    for row in &mut constant.control_points {
        for point in row {
            *point = Point3::new(4.0, -2.0, 1.0);
        }
    }
    let certificate = complete(
        project_surface_certified(
            &constant,
            Point3::new(0.0, 0.0, 2.0),
            options(1e-8, 1.0, 100_000, 8),
        )
        .unwrap(),
    );
    assert!(certificate.possible_minimizer_boxes.len() > 1);
    assert!(contains(&certificate, 2.0, -3.0));
    assert!(contains(&certificate, 5.0, 1.0));
    let covered_area: f64 = certificate
        .possible_minimizer_boxes
        .iter()
        .map(|candidate| {
            (candidate.u.end - candidate.u.start) * (candidate.v.end - candidate.v.start)
        })
        .sum();
    assert!((covered_area - 12.0).abs() <= 1e-12);
}

#[test]
fn global_lower_bound_is_below_every_independent_scalar_sample() {
    let surface = multispan_plane();
    let target = Point3::new(0.2, -0.4, 1.3);
    let certificate = complete(
        project_surface_certified(&surface, target, options(1e-8, 1e-4, 100_000, 64)).unwrap(),
    );
    for u_index in 0..=20 {
        for v_index in 0..=20 {
            let u = 2.0 + 3.0 * f64::from(u_index) / 20.0;
            let v = -3.0 + 4.0 * f64::from(v_index) / 20.0;
            let distance = bspline_jet(&surface, u, v).unwrap().point.distance(target);
            assert!(certificate.distance_lower_bound <= distance);
        }
    }
}

#[test]
fn projection_is_deterministic() {
    let surface = multispan_plane();
    let target = Point3::new(0.37, -0.29, 0.83);
    let policy = options(1e-8, 1e-4, 100_000, 64);
    let first = project_surface_certified(&surface, target, policy).unwrap();
    let second = project_surface_certified(&surface, target, policy).unwrap();
    assert_eq!(first, second);
}

#[test]
fn depth_limit_returns_sound_unresolved_boxes() {
    let result = project_surface_certified(
        &plane(),
        Point3::new(0.37, -0.19, 2.0),
        options(1e-12, 1e-12, 100_000, 1),
    )
    .unwrap();
    match result {
        CertifiedSurfaceProjection3::Unresolved {
            certificate,
            reason: SurfaceProjectionUnresolvedReason::DepthLimit,
        } => {
            assert!(!certificate.possible_minimizer_boxes.is_empty());
            assert!(certificate.distance_lower_bound <= certificate.distance_upper_bound);
        }
        other => panic!("expected depth-limited unresolved result: {other:?}"),
    }
}

#[test]
fn adjacent_binary_parameters_return_no_progress_instead_of_looping() {
    let lo = 1.0_f64;
    let hi = f64::from_bits(lo.to_bits() + 1);
    let mut tiny = plane();
    tiny.u_knots = vec![lo, hi];
    tiny.v_knots = vec![2.0, f64::from_bits(2.0_f64.to_bits() + 1)];
    let result = project_surface_certified(
        &tiny,
        Point3::new(0.1, 0.2, 1.0),
        options(1e-8, f64::from_bits(1), 100_000, 64),
    )
    .unwrap();
    assert!(matches!(
        result,
        CertifiedSurfaceProjection3::Unresolved {
            reason: SurfaceProjectionUnresolvedReason::FloatingPointNoProgress,
            ..
        }
    ));
}

#[test]
fn rejects_invalid_geometry_and_target_but_classifies_closed_axes_as_unsupported() {
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut surface = plane();
        surface.weights = Some(vec![vec![1.0, 1.0], vec![1.0, weight]]);
        assert!(matches!(
            project_surface_certified(&surface, Point3::ZERO, options(1e-6, 1e-4, 100_000, 16)),
            Err(GeomError::InvalidInput(_))
        ));
    }
    let mut non_finite = plane();
    non_finite.control_points[0][0].x = f64::NAN;
    assert!(matches!(
        project_surface_certified(&non_finite, Point3::ZERO, options(1e-6, 1e-4, 100_000, 16)),
        Err(GeomError::InvalidInput(_))
    ));
    for target in [
        Point3::new(f64::NAN, 0.0, 0.0),
        Point3::new(f64::INFINITY, 0.0, 0.0),
    ] {
        assert!(matches!(
            project_surface_certified(&plane(), target, options(1e-6, 1e-4, 100_000, 16)),
            Err(GeomError::InvalidInput(_))
        ));
    }
    for axis in [0, 1] {
        let mut closed = plane();
        if axis == 0 {
            closed.u_closed = true;
        } else {
            closed.v_closed = true;
        }
        assert!(matches!(
            project_surface_certified(&closed, Point3::ZERO, options(1e-6, 1e-4, 100_000, 16)),
            Err(GeomError::Unsupported { .. })
        ));
    }
}

#[test]
fn rejects_invalid_or_above_hard_cap_options() {
    let tolerance = Tolerance::new(1e-6, 1e-12).unwrap();
    for parameter in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(CertifiedSurfaceProjectionOptions::new(tolerance, parameter, 1, 1).is_err());
    }
    for work in [0, MAX_CERTIFIED_SURFACE_PROJECTION_WORK + 1] {
        assert!(CertifiedSurfaceProjectionOptions::new(tolerance, 1e-4, work, 1).is_err());
    }
    for depth in [0, MAX_CERTIFIED_SURFACE_PROJECTION_DEPTH + 1] {
        assert!(CertifiedSurfaceProjectionOptions::new(tolerance, 1e-4, 1, depth).is_err());
    }
}

#[test]
fn root_bound_and_representative_work_are_precharged() {
    let result = project_surface_certified(
        &plane(),
        Point3::new(0.25, -0.25, 1.0),
        options(100.0, 10.0, 40, 1),
    );
    assert!(matches!(result, Err(GeomError::BudgetExceeded { .. })));
}

#[test]
fn child_restriction_work_is_precharged_before_refinement() {
    let result = project_surface_certified(
        &plane(),
        Point3::new(0.25, -0.25, 1.0),
        options(1e-12, 1e-12, 100, 1),
    );
    assert!(matches!(result, Err(GeomError::BudgetExceeded { .. })));
}

#[test]
fn shared_refinement_search_budget_exhaustion_is_an_error() {
    let result = project_surface_certified(
        &multispan_plane(),
        Point3::new(0.13, -0.27, 2.0),
        options(1e-12, 1e-12, 50, 10),
    );
    assert!(matches!(result, Err(GeomError::BudgetExceeded { .. })));
}
