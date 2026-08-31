use axiolid_core::Point3;
use axiolid_curve::KnotSpec;
use axiolid_generate::{
    split_surface_pair_certified, CertifiedSurfacePairSplit3, CertifiedSurfacePairSplitOptions,
    SurfacePairMember, SurfacePairSplitUnresolvedReason,
};
use axiolid_nurbs::{
    CertifiedSurfaceSurfaceIntersection3, CertifiedSurfaceSurfaceIntersectionOptions,
};
use axiolid_surface::BSplineSurface;
use axiolid_topology::{audit_brep, Orientation};

fn xy_plane(x_min: f64, x_max: f64) -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(x_min, -1.0, 0.0), Point3::new(x_min, 1.0, 0.0)],
            vec![Point3::new(x_max, -1.0, 0.0), Point3::new(x_max, 1.0, 0.0)],
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

fn xz_plane(x_min: f64, x_max: f64) -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(x_min, 0.0, -1.0), Point3::new(x_min, 0.0, 1.0)],
            vec![Point3::new(x_max, 0.0, -1.0), Point3::new(x_max, 0.0, 1.0)],
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

fn options() -> CertifiedSurfacePairSplitOptions {
    CertifiedSurfacePairSplitOptions::new(
        CertifiedSurfaceSurfaceIntersectionOptions::new(1.0e-7, 100_000, 100_000, 64)
            .expect("valid intersection policy"),
        1.0e-6,
    )
    .expect("valid split policy")
}

#[test]
fn splits_boundary_owned_face_and_embeds_same_edge_in_containing_face() {
    let first = xy_plane(-1.0, 1.0);
    let second = xz_plane(-0.5, 0.5);
    let result = split_surface_pair_certified(&first, &second, options())
        .expect("certified topology integration succeeds");

    let split = match result {
        CertifiedSurfacePairSplit3::Split(split) => split,
        other => panic!("expected a complete trimmed split, got {other:?}"),
    };
    assert_eq!(split.visited_patch_pairs, 1);
    assert_eq!(split.boundary_queries, 8);
    assert!(split.max_surface_residual_upper_bound <= 1.0e-6);
    assert_eq!(split.brep.surfaces().len(), 2);
    assert_eq!(split.brep.topology().faces().len(), 3);
    assert_eq!(split.brep.topology().loops().len(), 3);
    assert_eq!(split.split_faces.len(), 2);
    assert_eq!(
        split.split_surface,
        axiolid_generate::SurfacePairMember::Second
    );
    assert_eq!(split.embedded_curve.face, split.unsplit_face);
    assert_eq!(split.embedded_curve.edge, split.intersection_edge);
    assert_eq!(split.embedded_curve.interval, axiolid_core::Interval::UNIT);
    assert!(split
        .brep
        .curves2()
        .get(split.embedded_curve.pcurve.index())
        .is_some());

    let health = audit_brep(split.brep.topology());
    assert!(
        health.is_tessellable(),
        "invalid split topology: {health:?}"
    );
    assert_eq!(health.dangling_references, 0);
    assert_eq!(health.open_loops, 0);

    let mut shared_uses = 0;
    let mut forward_uses = 0;
    let mut reverse_uses = 0;
    for (loop_index, loop_) in split.brep.topology().loops().iter().enumerate() {
        let loop_id = split
            .brep
            .topology()
            .loop_id_at(loop_index)
            .expect("enumerated loop exists");
        for (use_index, edge_use) in loop_.edges.iter().enumerate() {
            assert!(edge_use.pcurve.is_some());
            assert!(split.brep.pcurve_interval(loop_id, use_index).is_some());
            if edge_use.edge == split.intersection_edge {
                shared_uses += 1;
                match edge_use.orientation {
                    Orientation::Forward => forward_uses += 1,
                    Orientation::Reversed => reverse_uses += 1,
                }
            }
        }
    }
    assert_eq!(shared_uses, 2);
    assert_eq!(forward_uses, 1);
    assert_eq!(reverse_uses, 1);
    assert_eq!(
        split.brep.edge_interval(split.intersection_edge),
        Some(axiolid_core::Interval::UNIT)
    );

    for endpoint in [&split.trace.start, &split.trace.end] {
        for interval in [
            endpoint.parameters.first_u,
            endpoint.parameters.first_v,
            endpoint.parameters.second_u,
            endpoint.parameters.second_v,
        ] {
            assert!(interval.start.is_finite());
            assert!(interval.end.is_finite());
            assert!(interval.start <= interval.end);
        }
        assert!(endpoint.parameters.first_u.start >= 10.0);
        assert!(endpoint.parameters.first_u.end <= 12.0);
        assert!(endpoint.parameters.first_v.start >= -4.0);
        assert!(endpoint.parameters.first_v.end <= 0.0);
        assert!(endpoint.parameters.second_u.start >= 20.0);
        assert!(endpoint.parameters.second_u.end <= 24.0);
        assert!(endpoint.parameters.second_v.start >= 3.0);
        assert!(endpoint.parameters.second_v.end <= 7.0);
    }
}

#[test]
fn disjoint_surfaces_produce_complete_empty_split_without_brep() {
    let first = xy_plane(-1.0, 1.0);
    let mut second = xy_plane(-1.0, 1.0);
    for row in &mut second.control_points {
        for point in row {
            point.z = 2.0;
        }
    }
    let result = split_surface_pair_certified(&first, &second, options())
        .expect("disjoint certified query succeeds");
    assert!(matches!(
        result,
        CertifiedSurfacePairSplit3::Empty {
            visited_patch_pairs: 1,
            boundary_queries: 0,
        }
    ));
}

#[test]
fn swapped_inputs_preserve_geometric_owner_and_reverse_member_label() {
    let containing = xy_plane(-1.0, 1.0);
    let owner = xz_plane(-0.5, 0.5);
    let result = split_surface_pair_certified(&owner, &containing, options())
        .expect("swapped certified query succeeds");
    let CertifiedSurfacePairSplit3::Split(split) = result else {
        panic!("expected swapped split result");
    };
    assert_eq!(split.split_surface, SurfacePairMember::First);
    assert_eq!(split.brep.topology().faces().len(), 3);
    assert_eq!(split.embedded_curve.face, split.unsplit_face);
}

#[test]
fn certified_trace_above_residual_policy_stays_unresolved() {
    let first = xy_plane(-1.0, 1.0);
    let second = xz_plane(-0.5, 0.5);
    let options = CertifiedSurfacePairSplitOptions::new(
        CertifiedSurfaceSurfaceIntersectionOptions::default(),
        f64::MIN_POSITIVE,
    )
    .expect("positive finite policy");
    let result =
        split_surface_pair_certified(&first, &second, options).expect("valid query terminates");
    assert!(matches!(
        result,
        CertifiedSurfacePairSplit3::Unresolved {
            reason: SurfacePairSplitUnresolvedReason::ResidualExceedsPolicy,
            ..
        }
    ));
}

#[test]
fn dual_boundary_case_stays_unresolved_until_boundary_roots_are_certified() {
    let first = xy_plane(-1.0, 1.0);
    let second = xz_plane(-1.0, 1.0);
    let result = split_surface_pair_certified(&first, &second, options())
        .expect("valid asymmetric query terminates");
    match result {
        CertifiedSurfacePairSplit3::Unresolved {
            reason: SurfacePairSplitUnresolvedReason::IntersectionUnresolved,
            intersection: CertifiedSurfaceSurfaceIntersection3::Unresolved { traces, .. },
        } => assert!(traces.is_empty()),
        other => panic!("expected conservative boundary-root refusal, got {other:?}"),
    }
}

#[test]
fn mixed_endpoint_ownership_refuses_open_trim_chains() {
    let first = xy_plane(-1.0, 0.5);
    let second = xz_plane(-0.5, 1.0);
    let result = split_surface_pair_certified(&first, &second, options())
        .expect("valid mixed-ownership query terminates");
    match result {
        CertifiedSurfacePairSplit3::Unresolved {
            reason: SurfacePairSplitUnresolvedReason::UnsupportedEndpointOwnership,
            intersection: CertifiedSurfaceSurfaceIntersection3::Complete { traces, .. },
        } => assert_eq!(traces.len(), 1),
        other => panic!("expected mixed-ownership refusal, got {other:?}"),
    }
}

#[test]
fn split_policy_rejects_non_finite_or_non_positive_residual_limit() {
    let intersection = CertifiedSurfaceSurfaceIntersectionOptions::default();
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(CertifiedSurfacePairSplitOptions::new(intersection, invalid).is_err());
    }
}
