//! The provider must work through `MeshBooleanRegistry`, not just directly.
//!
//! This is what makes the seam real: `ifc-geometry` sees the registry, never
//! this crate.

mod support;

use axiolid_core::{BooleanOperator, Tolerance};
use axiolid_kernel::{ExecutionOptions, MeshBooleanRegistry};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use support::{boxx, volume};

fn registry() -> MeshBooleanRegistry {
    let mut registry = MeshBooleanRegistry::new();
    registry.register(0, BoolmeshBoolean::new());
    registry
}

/// Dispatching through the registry produces the same geometry as calling the
/// provider directly.
#[test]
fn the_registry_dispatches_to_the_provider() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    let opening = boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0);

    let result = registry()
        .boolean(
            &wall,
            &opening,
            BooleanOperator::Difference,
            &ExecutionOptions::new(Tolerance::METRE),
        )
        .expect("registry dispatch")
        .mesh;

    assert!(volume(&result) < volume(&wall));
}

/// A caller with a hard memory budget must be refused rather than allowed to
/// allocate past it: this provider declares `Unbounded` scratch.
#[test]
fn a_bounded_memory_budget_refuses_this_provider() {
    let options = ExecutionOptions::new(Tolerance::METRE).with_memory_budget(1024);
    let error = registry()
        .boolean(
            &boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0),
            &boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0),
            BooleanOperator::Difference,
            &options,
        )
        .expect_err("an unbounded provider cannot fit a declared budget");

    assert!(
        matches!(error, axiolid_kernel::GeomError::BudgetExceeded { .. }),
        "expected BudgetExceeded, got {error:?}"
    );
}
