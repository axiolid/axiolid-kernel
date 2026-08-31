use axiolid_core::Point3;
use axiolid_curve::{BSplineCurve3, KnotSpec};
use axiolid_nurbs::{
    intersect_curve_surface_certified, CertifiedCurveSurfaceIntersection3,
    CertifiedCurveSurfaceIntersectionOptions,
};
use axiolid_scalar::{curve::bspline_jet3, surface::bspline_jet};
use axiolid_surface::BSplineSurface;

fn line_through_plane() -> BSplineCurve3 {
    BSplineCurve3 {
        degree: 1,
        control_points: vec![Point3::new(0.25, -0.5, -1.0), Point3::new(0.25, -0.5, 2.0)],
        knots: vec![-2.0, 2.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::Unspecified,
        closed: false,
        self_intersect: None,
    }
}

fn rational_plane() -> BSplineSurface {
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
        weights: Some(vec![vec![1.0, 1.25], vec![0.75, 1.5]]),
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: None,
    }
}

#[test]
fn certifies_transverse_curve_surface_root_in_native_parameters() {
    let tolerance = 1.0e-6;
    let result = intersect_curve_surface_certified(
        &line_through_plane(),
        &rational_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(tolerance, 100_000, 64)
            .expect("valid options"),
    )
    .expect("certified query succeeds");

    let (intersections, visited_nodes) = match result {
        CertifiedCurveSurfaceIntersection3::Complete {
            intersections,
            visited_nodes,
        } => (intersections, visited_nodes),
        other => panic!("expected complete result, got {other:?}"),
    };
    assert!(visited_nodes > 0);
    assert_eq!(intersections.len(), 1);
    let root = &intersections[0];
    assert!(root.curve_parameter.end - root.curve_parameter.start <= tolerance);
    assert!(root.surface_u_parameter.end - root.surface_u_parameter.start <= tolerance);
    assert!(root.surface_v_parameter.end - root.surface_v_parameter.start <= tolerance);
    let expected_curve_parameter = -2.0 / 3.0;
    assert!(root.curve_parameter.start <= expected_curve_parameter);
    assert!(root.curve_parameter.end >= expected_curve_parameter);
    assert!(root.jacobian_determinant_lower_bound > 0.0);
    assert!(root.residual_upper_bound.is_finite());
    let curve_parameter = root.curve_parameter.start * 0.5 + root.curve_parameter.end * 0.5;
    let surface_u = root.surface_u_parameter.start * 0.5 + root.surface_u_parameter.end * 0.5;
    let surface_v = root.surface_v_parameter.start * 0.5 + root.surface_v_parameter.end * 0.5;
    let curve_point = bspline_jet3(&line_through_plane(), curve_parameter)
        .expect("curve midpoint evaluates")
        .point;
    let surface_point = bspline_jet(&rational_plane(), surface_u, surface_v)
        .expect("surface midpoint evaluates")
        .point;
    assert!(curve_point.distance(surface_point) <= root.residual_upper_bound);
}

#[test]
fn disjoint_curve_and_surface_are_completely_excluded() {
    let mut curve = line_through_plane();
    curve.control_points[0].z = 2.0;
    curve.control_points[1].z = 3.0;
    let result = intersect_curve_surface_certified(
        &curve,
        &rational_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 100_000, 64).unwrap(),
    )
    .expect("disjoint query succeeds");
    assert!(matches!(
        result,
        CertifiedCurveSurfaceIntersection3::Complete {
            intersections,
            ..
        } if intersections.is_empty()
    ));
}

#[test]
fn root_on_generated_subdivision_boundary_fails_closed() {
    let mut curve = line_through_plane();
    curve.control_points[1].z = 1.0;
    let result = intersect_curve_surface_certified(
        &curve,
        &rational_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 100_000, 32).unwrap(),
    )
    .expect("boundary query terminates");
    assert!(matches!(
        result,
        CertifiedCurveSurfaceIntersection3::Unresolved {
            candidate_boxes,
            ..
        } if !candidate_boxes.is_empty()
    ));
}

#[test]
fn isolated_tangential_contact_fails_closed_as_unresolved() {
    let mut curve = line_through_plane();
    curve.degree = 2;
    curve.control_points = vec![
        Point3::new(0.25, -0.5, 1.44),
        Point3::new(0.25, -0.5, -0.96),
        Point3::new(0.25, -0.5, 0.64),
    ];
    curve.knots = vec![-1.0, 1.0];
    curve.multiplicities = vec![3, 3];
    let result = intersect_curve_surface_certified(
        &curve,
        &rational_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-3, 10_000, 6).unwrap(),
    )
    .expect("tangential query terminates");
    assert!(matches!(
        result,
        CertifiedCurveSurfaceIntersection3::Unresolved {
            candidate_boxes,
            ..
        } if !candidate_boxes.is_empty()
    ));
}

#[test]
fn options_and_shared_work_budget_are_bounded() {
    assert!(CertifiedCurveSurfaceIntersectionOptions::new(0.0, 1, 1).is_err());
    assert!(CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 100_001, 1).is_err());
    assert!(CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 1, 65).is_err());

    let error = intersect_curve_surface_certified(
        &line_through_plane(),
        &rational_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 1, 1).unwrap(),
    )
    .expect_err("refinement cannot fit one work unit");
    assert!(matches!(
        error,
        axiolid_kernel::GeomError::BudgetExceeded { .. }
    ));
}

#[test]
fn nonpositive_rational_surface_weight_is_rejected() {
    let mut surface = rational_plane();
    surface.weights.as_mut().unwrap()[1][1] = 0.0;
    assert!(intersect_curve_surface_certified(
        &line_through_plane(),
        &surface,
        CertifiedCurveSurfaceIntersectionOptions::default(),
    )
    .is_err());
}

fn two_span_plane() -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
            vec![Point3::new(0.0, -1.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![10.0, 11.0, 12.0],
        u_multiplicities: vec![2, 1, 2],
        v_knots: vec![-4.0, 0.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::Unspecified,
        self_intersect: Some(false),
    }
}

#[test]
fn multispan_surface_returns_only_corresponding_patch_root() {
    let mut curve = line_through_plane();
    for point in &mut curve.control_points {
        point.x = -0.5;
    }
    let result = intersect_curve_surface_certified(
        &curve,
        &two_span_plane(),
        CertifiedCurveSurfaceIntersectionOptions::new(1.0e-6, 100_000, 64).unwrap(),
    )
    .expect("query succeeds");
    let CertifiedCurveSurfaceIntersection3::Complete { intersections, .. } = result else {
        panic!("transverse root must be complete");
    };
    assert_eq!(intersections.len(), 1);
    let root = &intersections[0];
    assert!(root.surface_u_parameter.start <= 10.5);
    assert!(root.surface_u_parameter.end >= 10.5);
    assert!(root.surface_u_parameter.end < 11.0);
}
