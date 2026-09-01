use axiolid_core::Point3;
use axiolid_curve::KnotSpec;
use axiolid_nurbs::{
    intersect_surface_surface_certified, CertifiedSurfaceSurfaceIntersection3,
    CertifiedSurfaceSurfaceIntersectionOptions,
};
use axiolid_reference::surface::bspline_jet;
use axiolid_surface::BSplineSurface;

fn xy_plane() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
            vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![10.0, 12.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![-4.0, 0.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: None,
    }
}

fn xz_plane() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-0.5, 0.0, -1.0), Point3::new(-0.5, 0.0, 1.0)],
            vec![Point3::new(0.5, 0.0, -1.0), Point3::new(0.5, 0.0, 1.0)],
        ],
        u_knots: vec![20.0, 24.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![3.0, 7.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: None,
    }
}

#[test]
fn certifies_complete_transverse_affine_trace_in_native_parameters() {
    let tolerance = 1.0e-6;
    let first = xy_plane();
    let second = xz_plane();
    let result = intersect_surface_surface_certified(
        &first,
        &second,
        CertifiedSurfaceSurfaceIntersectionOptions::new(tolerance, 100_000, 100_000, 64)
            .expect("valid options"),
    )
    .expect("certified query succeeds");

    let (traces, visited_patch_pairs, boundary_queries) = match result {
        CertifiedSurfaceSurfaceIntersection3::Complete {
            traces,
            visited_patch_pairs,
            boundary_queries,
        } => (traces, visited_patch_pairs, boundary_queries),
        other => panic!("expected complete trace, got {other:?}"),
    };
    assert_eq!(visited_patch_pairs, 1);
    assert_eq!(boundary_queries, 8);
    assert_eq!(traces.len(), 1);

    let trace = &traces[0];
    assert!(trace.normal_cross_squared_lower_bound > 0.0);
    assert!(trace.normal_cross_squared_lower_bound <= 0.00390625);
    let mut endpoint_x = Vec::new();
    for endpoint in [&trace.start, &trace.end] {
        for interval in [
            endpoint.parameters.first_u,
            endpoint.parameters.first_v,
            endpoint.parameters.second_u,
            endpoint.parameters.second_v,
        ] {
            assert!(interval.end - interval.start <= tolerance);
        }
        assert!(endpoint.residual_upper_bound.is_finite());
        let first_u =
            endpoint.parameters.first_u.start * 0.5 + endpoint.parameters.first_u.end * 0.5;
        let first_v =
            endpoint.parameters.first_v.start * 0.5 + endpoint.parameters.first_v.end * 0.5;
        let second_u =
            endpoint.parameters.second_u.start * 0.5 + endpoint.parameters.second_u.end * 0.5;
        let second_v =
            endpoint.parameters.second_v.start * 0.5 + endpoint.parameters.second_v.end * 0.5;
        let first_point = bspline_jet(&first, first_u, first_v)
            .expect("first endpoint evaluates")
            .point;
        let second_point = bspline_jet(&second, second_u, second_v)
            .expect("second endpoint evaluates")
            .point;
        assert!(first_point.distance(second_point) <= endpoint.residual_upper_bound);
        endpoint_x.push(endpoint.point.x);
    }
    endpoint_x.sort_by(f64::total_cmp);
    assert!((endpoint_x[0] + 0.5).abs() <= tolerance);
    assert!((endpoint_x[1] - 0.5).abs() <= tolerance);
}

#[test]
fn disjoint_surface_hulls_are_completely_excluded() {
    let first = xy_plane();
    let mut second = xy_plane();
    for row in &mut second.control_points {
        for point in row {
            point.z = 2.0;
        }
    }
    let result = intersect_surface_surface_certified(
        &first,
        &second,
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("disjoint query succeeds");
    assert!(matches!(
        result,
        CertifiedSurfaceSurfaceIntersection3::Complete { traces, .. } if traces.is_empty()
    ));
}

#[test]
fn non_affine_surface_pair_remains_unresolved() {
    let first = xy_plane();
    let mut second = xz_plane();
    second.control_points[1][1].y = 0.25;
    let result = intersect_surface_surface_certified(
        &first,
        &second,
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("non-affine query terminates");
    assert!(matches!(
        result,
        CertifiedSurfaceSurfaceIntersection3::Unresolved {
            traces,
            candidate_boxes,
            ..
        } if traces.is_empty() && !candidate_boxes.is_empty()
    ));
}

#[test]
fn coincident_affine_surfaces_remain_unresolved() {
    let surface = xy_plane();
    let result = intersect_surface_surface_certified(
        &surface,
        &surface,
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("coincident query terminates");
    assert!(matches!(
        result,
        CertifiedSurfaceSurfaceIntersection3::Unresolved {
            traces,
            candidate_boxes,
            ..
        } if traces.is_empty() && !candidate_boxes.is_empty()
    ));
}

#[test]
fn options_and_refinement_work_are_bounded() {
    assert!(CertifiedSurfaceSurfaceIntersectionOptions::new(0.0, 1, 1, 1).is_err());
    assert!(CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 100_001, 1, 1).is_err());
    assert!(CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 1, 100_001, 1).is_err());
    assert!(CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 1, 1, 65).is_err());

    let error = intersect_surface_surface_certified(
        &xy_plane(),
        &xz_plane(),
        CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-6, 1, 1, 1).unwrap(),
    )
    .expect_err("one work unit cannot refine both surfaces");
    assert!(matches!(
        error,
        axiolid_contracts::GeomError::BudgetExceeded { .. }
    ));
}

#[test]
fn exact_dyadic_affine_check_rejects_rounded_near_parallelogram() {
    let mut second = xz_plane();
    second.control_points = vec![
        vec![
            Point3::new(0.00038395656557033093, 0.0, -1.0),
            Point3::new(1.1935775435669205e-20, 0.0, 1.0),
        ],
        vec![
            Point3::new(5.941960404966601e-11, 0.0, -1.0),
            Point3::new(-0.0003839565061507269, 0.0, 1.0),
        ],
    ];
    // Left-associative binary64 evaluates `p11 - p10 - p01 + p00` as zero,
    // while the exact dyadic sum is nonzero. The stored net is therefore not affine.

    let result = intersect_surface_surface_certified(
        &xy_plane(),
        &second,
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("near-affine query terminates");
    assert!(matches!(
        result,
        CertifiedSurfaceSurfaceIntersection3::Unresolved { traces, .. }
            if traces.is_empty()
    ));
}

#[test]
fn trace_on_patch_boundary_remains_unresolved() {
    let mut second = xz_plane();
    for row in &mut second.control_points {
        for point in row {
            point.y = -1.0;
        }
    }
    let result = intersect_surface_surface_certified(
        &xy_plane(),
        &second,
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("boundary-owned query terminates");
    assert!(matches!(
        result,
        CertifiedSurfaceSurfaceIntersection3::Unresolved { traces, .. }
            if traces.is_empty()
    ));
}

#[test]
fn swapping_surfaces_preserves_trace_and_swaps_parameter_ownership() {
    let result = intersect_surface_surface_certified(
        &xz_plane(),
        &xy_plane(),
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect("swapped query succeeds");
    let trace = match result {
        CertifiedSurfaceSurfaceIntersection3::Complete { traces, .. } => {
            assert_eq!(traces.len(), 1);
            traces.into_iter().next().unwrap()
        }
        other => panic!("expected complete swapped trace, got {other:?}"),
    };
    for endpoint in [trace.start, trace.end] {
        assert!(
            endpoint.parameters.first_u.start == 20.0 || endpoint.parameters.first_u.start == 24.0
        );
        assert!(endpoint.parameters.second_u.start > 10.0);
        assert!(endpoint.parameters.second_u.end < 12.0);
    }
}

#[test]
fn full_multiplicity_internal_axis_is_unsupported() {
    let mut first = xy_plane();
    first.control_points = vec![
        vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
        vec![Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ];
    first.u_knots = vec![10.0, 11.0, 12.0];
    first.u_multiplicities = vec![2, 2, 2];
    let error = intersect_surface_surface_certified(
        &first,
        &xz_plane(),
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
    )
    .expect_err("full-multiplicity internal knot is outside the certified domain");
    assert!(matches!(
        error,
        axiolid_contracts::GeomError::InvalidInput(_)
    ));
}
