#![cfg(feature = "mesh-boolean")]

use axiolid_contracts::{ExecutionOptions, Precision};
use axiolid_core::Tolerance;

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::new(1.0e-9, 1.0e-9).unwrap()).with_precision(Precision::F64)
}

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionTarget, GeomError, GeomResult,
    ScratchRequirement,
};
use axiolid_core::BooleanOperator;
use axiolid_dispatch::MeshBooleanRegistry;
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_contract::{BooleanEvidence, BooleanOutcome, MeshBoolean};

/// Outward-oriented unit cube: the minimal admissible boolean operand.
fn admissible_cube() -> TriMesh {
    let positions = vec![
        [0.0, 0.0, 0.0].into(),
        [1.0, 0.0, 0.0].into(),
        [1.0, 1.0, 0.0].into(),
        [0.0, 1.0, 0.0].into(),
        [0.0, 0.0, 1.0].into(),
        [1.0, 0.0, 1.0].into(),
        [1.0, 1.0, 1.0].into(),
        [0.0, 1.0, 1.0].into(),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    TriMesh::new(positions, indices)
}
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Provider with a declared scratch bound that records whether it ran.
#[derive(Debug)]
struct Budgeted {
    id: BackendId,
    scratch: ScratchRequirement,
    calls: Arc<AtomicUsize>,
}

impl Backend for Budgeted {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(self.id, ExecutionTarget::PortableCpu)
    }
}

impl MeshBoolean for Budgeted {
    fn scratch_requirement(&self) -> ScratchRequirement {
        self.scratch
    }

    fn boolean(
        &self,
        subject: &TriMesh,
        _tool: &TriMesh,
        _operation: BooleanOperator,
        _options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(BooleanOutcome::new(
            subject.clone(),
            BooleanEvidence::default(),
        ))
    }
}

/// Operand for the budget tests.
///
/// Was a degenerate three-vertex sheet, which the contract now rejects
/// before dispatch. These tests are about budget routing, not
/// admissibility, so they need a mesh that actually gets that far.
fn mesh() -> TriMesh {
    admissible_cube()
}

/// The budget is only real if it blocks dispatch. A provider whose declared
/// scratch cannot fit must never be given the chance to allocate.
#[test]
fn an_over_budget_provider_is_never_invoked() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = MeshBooleanRegistry::new();
    registry.register(
        0,
        Budgeted {
            id: BackendId::new("greedy"),
            scratch: ScratchRequirement::Fixed { bytes: 4096 },
            calls: Arc::clone(&calls),
        },
    );

    let error = registry
        .boolean(
            &mesh(),
            &mesh(),
            BooleanOperator::Difference,
            &options().with_memory_budget(16),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        GeomError::BudgetExceeded { resource: "memory" }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "provider must not run");
}

/// An over-budget provider must not poison the registry: a leaner provider
/// registered behind it still runs, exactly like the Unsupported/Unavailable
/// fallback path.
#[test]
fn a_leaner_provider_still_runs_after_an_over_budget_one() {
    let greedy_calls = Arc::new(AtomicUsize::new(0));
    let lean_calls = Arc::new(AtomicUsize::new(0));
    let mut registry = MeshBooleanRegistry::new();
    registry.register(
        10,
        Budgeted {
            id: BackendId::new("greedy"),
            scratch: ScratchRequirement::Fixed { bytes: 4096 },
            calls: Arc::clone(&greedy_calls),
        },
    );
    registry.register(
        0,
        Budgeted {
            id: BackendId::new("lean"),
            scratch: ScratchRequirement::None,
            calls: Arc::clone(&lean_calls),
        },
    );

    registry
        .boolean(
            &mesh(),
            &mesh(),
            BooleanOperator::Difference,
            &options().with_memory_budget(16),
        )
        .expect("the lean provider fits the budget");

    assert_eq!(greedy_calls.load(Ordering::SeqCst), 0);
    assert_eq!(lean_calls.load(Ordering::SeqCst), 1);
}
