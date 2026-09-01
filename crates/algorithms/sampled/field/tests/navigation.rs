//! Geometry-only traversal primitives. Requires the `navigation` feature.
#![cfg(feature = "navigation")]

mod fixtures;

use axiolid_core::Vec3;
use axiolid_field::navigate::{RouteOutcome, TraversalEnvelope, TraversalGraph};
use axiolid_field::{sample_triangles_cpu, LayeredFieldError, Triangle3};

fn envelope(radius: f64, height: f64, step: f64, slope: f64) -> TraversalEnvelope {
    TraversalEnvelope {
        agent_radius: radius,
        agent_height: height,
        max_step: step,
        max_slope: slope,
    }
}

/// A flat deck at w = 0 spanning the whole field, plus an optional raised
/// terrace so step/slope limits have something real to reject.
fn deck(x0: f64, x1: f64, w: f64) -> [Triangle3; 2] {
    let p = |x: f64, y: f64| Vec3::new(x, y, w);
    [
        Triangle3::new(p(x0, -1.0), p(x1, -1.0), p(x1, 6.0)),
        Triangle3::new(p(x0, -1.0), p(x1, 6.0), p(x0, 6.0)),
    ]
}

#[test]
fn a_flat_deck_yields_one_component_and_a_route() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(5.0, 1.0, 6.0), 1.0);
    let field = sample_triangles_cpu(&config, &deck(-1.0, 6.0, 0.0)).unwrap();
    let graph = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.1, 1.0)).unwrap();

    assert_eq!(graph.evidence().nodes, 5);
    assert_eq!(graph.evidence().components, 1);
    assert!(graph.connected((0, 0), (4, 0)));

    match graph.find_route((0, 0), (4, 0)).unwrap() {
        RouteOutcome::Route { nodes, length, .. } => {
            assert_eq!(nodes.len(), 5);
            assert_eq!(nodes.first().unwrap().x, 0);
            assert_eq!(nodes.last().unwrap().x, 4);
            assert!((length - 4.0).abs() < 1e-9, "length {length}");
        }
        other => panic!("expected a route, got {other:?}"),
    }
}

#[test]
fn a_gap_reports_no_route_under_this_envelope() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(5.0, 1.0, 6.0), 1.0);
    // Two decks with the middle cell centre uncovered.
    let mut geometry = deck(-1.0, 1.2, 0.0).to_vec();
    geometry.extend(deck(2.8, 6.0, 0.0));
    let field = sample_triangles_cpu(&config, &geometry).unwrap();
    let graph = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.1, 1.0)).unwrap();

    assert_eq!(graph.evidence().components, 2);
    assert!(!graph.connected((0, 0), (4, 0)));

    match graph.find_route((0, 0), (4, 0)).unwrap() {
        // The verdict is geometric: no route under this envelope. It is not a
        // statement about accessibility, egress, or any rule.
        RouteOutcome::NoRouteUnderEnvelope { evidence } => {
            assert_eq!(evidence.components, 2);
            assert!(evidence.nodes > 0);
        }
        other => panic!("expected no route, got {other:?}"),
    }
}

#[test]
fn max_step_rejects_an_abrupt_layer_change_with_evidence() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 1.0, 8.0), 1.0);
    // Left half at w = 0, right half at w = 1: a one-metre step.
    let mut geometry = deck(-1.0, 2.0, 0.0).to_vec();
    geometry.extend(deck(2.0, 5.0, 1.0));
    let field = sample_triangles_cpu(&config, &geometry).unwrap();

    let permissive = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 1.5, 2.0)).unwrap();
    assert!(permissive.connected((0, 0), (3, 0)));

    let strict = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.2, 2.0)).unwrap();
    assert!(strict.evidence().rejected_by_step >= 1);
    assert!(!strict.connected((0, 0), (3, 0)));
    assert!(matches!(
        strict.find_route((0, 0), (3, 0)).unwrap(),
        RouteOutcome::NoRouteUnderEnvelope { .. }
    ));
}

#[test]
fn max_slope_rejects_a_gradient_with_evidence() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 1.0, 8.0), 1.0);
    let mut geometry = deck(-1.0, 2.0, 0.0).to_vec();
    geometry.extend(deck(2.0, 5.0, 0.5));
    let field = sample_triangles_cpu(&config, &geometry).unwrap();

    // 0.5 rise over a 1.0 run is a slope of 0.5.
    let ok = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 1.0, 0.6)).unwrap();
    assert!(ok.connected((0, 0), (3, 0)));

    let steep = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 1.0, 0.25)).unwrap();
    assert!(steep.evidence().rejected_by_slope >= 1);
    assert!(!steep.connected((0, 0), (3, 0)));
}

#[test]
fn agent_height_filters_supports_without_naming_a_rule() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(3.0, 1.0, 10.0), 1.0);
    // Floor at 0 everywhere, ceiling at 1.0 over the middle cell only.
    let mut geometry = deck(-1.0, 4.0, 0.0).to_vec();
    geometry.extend(deck(0.9, 2.1, 1.0));
    let field = sample_triangles_cpu(&config, &geometry).unwrap();

    let low = TraversalGraph::build(&field, &config, &envelope(0.0, 0.5, 1.0, 2.0)).unwrap();
    assert_eq!(low.evidence().rejected_by_height, 0);

    let tall = TraversalGraph::build(&field, &config, &envelope(0.0, 2.0, 1.0, 2.0)).unwrap();
    assert!(
        tall.evidence().rejected_by_height >= 1,
        "a 2 m envelope must not fit under a 1 m ceiling"
    );
    assert!(!tall.connected((0, 0), (2, 0)));
}

#[test]
fn agent_radius_inflates_obstacles_and_is_reported() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 3.0, 6.0), 1.0);
    // A deck missing its last column: the void inflates inward.
    let field = sample_triangles_cpu(&config, &deck(-1.0, 2.2, 0.0)).unwrap();

    let tight = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 1.0, 2.0)).unwrap();
    let inflated = TraversalGraph::build(&field, &config, &envelope(1.0, 0.0, 1.0, 2.0)).unwrap();

    assert!(
        inflated.evidence().nodes < tight.evidence().nodes,
        "inflation must remove supports adjacent to the void"
    );
    assert!(inflated.evidence().rejected_by_radius >= 1);
}

#[test]
fn routes_are_deterministic_across_repeated_queries() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 4.0, 6.0), 1.0);
    let field = sample_triangles_cpu(&config, &deck(-1.0, 6.0, 0.0)).unwrap();
    let graph = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.1, 1.0)).unwrap();

    // A square grid has many equal-length routes; the tie-break must pick the
    // same one every time.
    let first = graph.find_route((0, 0), (3, 3)).unwrap();
    for _ in 0..8 {
        assert_eq!(graph.find_route((0, 0), (3, 3)).unwrap(), first);
    }
    let rebuilt = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.1, 1.0)).unwrap();
    assert_eq!(rebuilt.find_route((0, 0), (3, 3)).unwrap(), first);

    match first {
        RouteOutcome::Route { length, .. } => assert!((length - 6.0).abs() < 1e-9),
        other => panic!("expected a route, got {other:?}"),
    }
}

#[test]
fn traversal_validates_envelope_and_endpoints() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(3.0, 1.0, 6.0), 1.0);
    let field = sample_triangles_cpu(&config, &deck(-1.0, 4.0, 0.0)).unwrap();

    assert_eq!(
        TraversalGraph::build(&field, &config, &envelope(-1.0, 0.0, 0.1, 1.0))
            .expect_err("negative radius"),
        LayeredFieldError::InvalidEnvelope
    );
    assert_eq!(
        TraversalGraph::build(&field, &config, &envelope(0.0, f64::NAN, 0.1, 1.0))
            .expect_err("non-finite height"),
        LayeredFieldError::InvalidEnvelope
    );

    let graph = TraversalGraph::build(&field, &config, &envelope(0.0, 0.0, 0.1, 1.0)).unwrap();
    assert_eq!(
        graph.find_route((0, 0), (99, 99)),
        Err(LayeredFieldError::NodeOutsideField)
    );
    assert!(!graph.connected((0, 0), (99, 99)));
}

#[test]
fn multi_layer_field_keeps_levels_independent() {
    let frame = fixtures::z_up_frame();
    let config = fixtures::config(frame, Vec3::new(4.0, 1.0, 12.0), 1.0);
    // Two decks stacked over the same cells: a single-floor grid could not
    // represent this at all.
    let mut geometry = deck(-1.0, 5.0, 0.0).to_vec();
    geometry.extend(deck(-1.0, 5.0, 4.0));
    let field = sample_triangles_cpu(&config, &geometry).unwrap();
    assert_eq!(field.cell(0, 0).unwrap().surfaces().len(), 2);

    // Traversal uses the lowest support, and the 4 m upper deck leaves ample
    // clearance, so the lower level stays fully connected.
    let graph = TraversalGraph::build(&field, &config, &envelope(0.0, 2.0, 0.1, 1.0)).unwrap();
    assert_eq!(graph.evidence().components, 1);
    assert_eq!(graph.node(0, 0).unwrap().w, 0.0);
}
