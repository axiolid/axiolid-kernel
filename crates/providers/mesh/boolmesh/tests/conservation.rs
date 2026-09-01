//! Gate 2: volume conservation, the ADR 0003 validation criterion.
//!
//! `vol(a \ b) + vol(a n b) == vol(a)` is triangulation-invariant: it holds
//! whatever indices the backend emits, so it tests the *geometry* rather than
//! an index buffer. That is what makes it a usable gate against an adopted
//! implementation whose output triangulation we do not control.

mod support;

use axiolid_boolmesh::BoolmeshBoolean;
use axiolid_core::{BooleanOperator, Tolerance};
use axiolid_kernel::{ExecutionOptions, MeshBoolean};
use axiolid_mesh::TriMesh;
use support::{boxx, volume};

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

fn apply(subject: &TriMesh, tool: &TriMesh, op: BooleanOperator) -> TriMesh {
    BoolmeshBoolean::new()
        .boolean(subject, tool, op, &options())
        .unwrap_or_else(|error| panic!("{op:?} failed: {error}"))
        .mesh
}

/// The wall/opening case, checked by conservation rather than by index match.
#[test]
fn difference_and_intersection_partition_the_subject() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    let opening = boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0);

    let difference = apply(&wall, &opening, BooleanOperator::Difference);
    let intersection = apply(&wall, &opening, BooleanOperator::Intersection);

    let total = volume(&difference) + volume(&intersection);
    assert!(
        (total - volume(&wall)).abs() < 1e-9,
        "a\\b + a^b must equal a: {total} vs {}",
        volume(&wall)
    );
}

/// The same law must hold when the cutter is rotated off-axis, where no
/// axis-aligned fast path can apply.
#[test]
fn conservation_holds_for_rotated_cutters() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    for angle in [0.1, 0.6435011, -0.9272952, 1.2] {
        let opening = boxx(2.0, 0.1, 0.3, 1.0, 0.4, 1.2, angle);
        let difference = apply(&wall, &opening, BooleanOperator::Difference);
        let intersection = apply(&wall, &opening, BooleanOperator::Intersection);
        let total = volume(&difference) + volume(&intersection);
        assert!(
            (total - volume(&wall)).abs() < 1e-9,
            "conservation failed at angle {angle}: {total} vs {}",
            volume(&wall)
        );
    }
}

/// Union is the complementary law: `vol(a u b) == vol(a) + vol(b) - vol(a n b)`.
#[test]
fn union_accounts_for_the_shared_region_exactly_once() {
    let a = boxx(0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 0.0);
    let b = boxx(1.0, 1.0, 0.0, 2.0, 2.0, 2.0, 0.0);

    let union = apply(&a, &b, BooleanOperator::Union);
    let intersection = apply(&a, &b, BooleanOperator::Intersection);

    let expected = volume(&a) + volume(&b) - volume(&intersection);
    assert!(
        (volume(&union) - expected).abs() < 1e-9,
        "union {} vs expected {expected}",
        volume(&union)
    );
}
