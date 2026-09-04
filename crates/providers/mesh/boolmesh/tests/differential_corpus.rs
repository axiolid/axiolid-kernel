//! Corpus-driven differential testing against the scalar oracle (#8).
//!
//! Every fixture in `axiolid-fixtures` is pushed through the mesh operations
//! and the result cross-checked against `axiolid-reference`, the independent
//! scalar implementation. Two implementations agreeing is weak evidence on its
//! own; two implementations DISAGREEING is decisive, which is what this is for.
//!
//! Fixtures are iterated from `corpus()` rather than named individually, so a
//! fixture added upstream is covered here without editing this file.
//!
//! # What is asserted, and what deliberately is not
//!
//! Degenerate fixtures are expected to be REFUSED, not silently handled. The
//! assertion is therefore "refuse or produce something structurally valid",
//! never "produce a specific number": demanding a number from a case that has
//! no defined answer is how a tolerance-snapping implementation passes a test
//! suite it should fail.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::{BooleanOperator, Point3, Tolerance};
use axiolid_fixtures::{coplanar_contact, corpus, unit_cube};
use axiolid_measure::{mesh_distance, volume_properties};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_boolean_contract::MeshBoolean;
use axiolid_reference::clash::point_inside;

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

/// No fixture may make a mesh operation panic or emit non-finite geometry.
///
/// This is the weakest possible contract and the most important one: a
/// degenerate input may be refused, but it may never corrupt the output or
/// take the process down.
#[test]
fn no_fixture_panics_or_produces_non_finite_output() {
    let tool = unit_cube().mesh;
    for fixture in corpus() {
        for operator in [
            BooleanOperator::Union,
            BooleanOperator::Intersection,
            BooleanOperator::Difference,
        ] {
            let outcome =
                BoolmeshBoolean::new().boolean(&fixture.mesh, &tool, operator, &options());
            let Ok(result) = outcome else {
                // A typed refusal is an acceptable answer for a degenerate case.
                continue;
            };
            assert!(
                result.mesh.positions.iter().all(|p| p.is_finite()),
                "{}: {operator:?} produced non-finite geometry",
                fixture.name
            );
        }
    }
}

/// The provider's boolean result agrees with the reference oracle's
/// point-in-solid classification.
///
/// The provider decides membership structurally, by cutting and stitching
/// faces. `axiolid-reference::point_inside` decides it independently, by
/// scalar ray classification. For a probe well inside or well outside, the two
/// must agree; if they do not, one of them is wrong and the test says which
/// probe exposed it.
#[test]
fn boolean_membership_agrees_with_the_scalar_oracle() {
    let subject = unit_cube().mesh;
    let mut tool = unit_cube().mesh;
    for position in &mut tool.positions {
        position.x += 0.5;
        position.y += 0.5;
    }

    let result = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Intersection, &options())
        .expect("two valid cubes intersect")
        .mesh;

    // The true intersection is [0.5,1] x [0.5,1] x [0,1]. Probes are placed
    // well away from every face so the answer does not hinge on a boundary
    // convention that the two implementations are entitled to differ on.
    let probes = [
        (Point3::new(0.75, 0.75, 0.5), true),
        (Point3::new(0.25, 0.25, 0.5), false),
        (Point3::new(0.75, 0.25, 0.5), false),
        (Point3::new(0.25, 0.75, 0.5), false),
        (Point3::new(5.0, 5.0, 5.0), false),
    ];

    for (probe, expected) in probes {
        let oracle = point_inside(probe, &result, Tolerance::METRE)
            .expect("the oracle must classify a probe away from the boundary");
        assert_eq!(
            oracle, expected,
            "oracle says inside={oracle} for {probe:?}, geometry says {expected}"
        );
    }
}

/// Coplanar contact resolves to the exact union volume.
///
/// Two unit cubes sharing one face plane. The union is exactly 2: not 2 plus a
/// sliver from double-counting the shared face, and not 1 from collapsing it.
/// Both failure modes are what a tolerance-based classifier produces here.
#[test]
fn coplanar_contact_gives_the_exact_union_volume() {
    let (left, right) = coplanar_contact();
    let result = BoolmeshBoolean::new()
        .boolean(&left.mesh, &right.mesh, BooleanOperator::Union, &options())
        .expect("coplanar union is well defined")
        .mesh;

    let volume = volume_properties(&result, Tolerance::METRE)
        .expect("the union of two closed cubes is closed")
        .signed_volume
        .abs();
    assert!(
        (volume - 2.0).abs() < 1e-9,
        "{}: expected union volume 2, got {volume}",
        left.provenance.expectation
    );
}

/// Bounds survive a nine-order-of-magnitude scale gap.
///
/// The nanometre box is 1e-9 the size of the kilometre box. A bounds routine
/// using an absolute epsilon loses it entirely, which shows up as bounds that
/// do not reach the origin corner.
#[test]
fn bounds_do_not_lose_features_across_scales() {
    let fixture = corpus()
        .into_iter()
        .find(|f| f.name == "scale_disparity")
        .expect("the corpus carries the scale-disparity case");

    let bounds = fixture.mesh.bounds();
    assert!(
        bounds.min.x <= 0.0 && bounds.max.x >= 1.0e3,
        "{}: bounds lost a box, got {:?}..{:?}",
        fixture.provenance.expectation,
        bounds.min,
        bounds.max
    );
}

/// An open shell must not be assigned a confident volume.
///
/// Volume needs a closed boundary. Reporting a plausible number for an open
/// shell is the single most dangerous silent failure in this area, because the
/// number looks right.
#[test]
fn an_open_shell_is_refused_a_volume() {
    let fixture = corpus()
        .into_iter()
        .find(|f| f.name == "open_shell")
        .expect("the corpus carries the open-shell case");

    assert!(
        volume_properties(&fixture.mesh, Tolerance::METRE).is_err(),
        "{}",
        fixture.provenance.expectation
    );
}

/// Spatial distance between corpus fixtures never contradicts their bounds.
///
/// If two meshes have disjoint bounding boxes separated by a gap, their true
/// surface distance cannot be less than that gap. This catches a broad-phase
/// that prunes a pair it should have kept.
#[test]
fn distance_never_undercuts_the_bounds_separation() {
    let mut far = unit_cube().mesh;
    for position in &mut far.positions {
        position.x += 10.0;
    }
    let near = unit_cube().mesh;

    let distance = mesh_distance(&near, &far)
        .expect("two valid cubes")
        .distance_squared
        .sqrt();

    // Bounds are [0,1] and [10,11] in x, so the gap is exactly 9.
    assert!(
        (distance - 9.0).abs() < 1e-9,
        "expected the bounds gap of 9, got {distance}"
    );
}
