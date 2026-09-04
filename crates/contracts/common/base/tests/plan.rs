//! Plan admission: determinism, budgets, provenance (#19).
//!
//! The point of these is that a plan REFUSES rather than quietly delivering
//! something weaker than asked for. A test that only checks the happy path
//! would pass against an implementation that ignores the request entirely,
//! which is the exact bug this closes.

use axiolid_contracts::{
    BackendId, Determinism, ExecutionOptions, GeomError, Operation, Plan, PlanStep,
    ScratchRequirement,
};
use axiolid_core::Tolerance;

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

fn step(guaranteed: Determinism) -> PlanStep {
    PlanStep::new(
        Operation::MeshBoolean,
        BackendId::new("test"),
        guaranteed,
        "unit cube",
    )
}

/// A provider guaranteeing exactly what was asked is admitted.
#[test]
fn a_matching_guarantee_is_admitted() {
    let mut plan = Plan::new(options().with_determinism(Determinism::Topological));
    plan.admit(step(Determinism::Topological), ScratchRequirement::None, 0)
        .expect("an exact match must be admitted");
    assert_eq!(plan.steps().len(), 1);
}

/// A stronger provider is admitted: strength is ordered, not equality.
///
/// Rejecting a backend for being better than required is the mirror-image bug
/// of accepting one that is worse, and just as wrong.
#[test]
fn a_stronger_guarantee_is_admitted() {
    let mut plan = Plan::new(options().with_determinism(Determinism::Topological));
    plan.admit(step(Determinism::Bitwise), ScratchRequirement::None, 0)
        .expect("a stronger guarantee must satisfy a weaker request");
    assert_eq!(plan.steps()[0].guaranteed, Determinism::Bitwise);
}

/// A weaker provider is REFUSED, not silently downgraded.
///
/// This is the hole the plan closes. Before it, a caller could request
/// `Bitwise` -- documented as the only level supporting hashing a result or
/// comparing artifacts across machines -- and receive best-effort output with
/// no signal the request was dropped. Nothing in the kernel read the field.
#[test]
fn a_weaker_guarantee_is_refused_rather_than_downgraded() {
    let mut plan = Plan::new(options().with_determinism(Determinism::Bitwise));
    let error = plan
        .admit(step(Determinism::BestEffort), ScratchRequirement::None, 0)
        .expect_err("a best-effort backend cannot satisfy a bitwise plan");

    assert!(
        matches!(error, GeomError::BackendContractViolation { .. }),
        "expected a contract violation naming the gap, got {error:?}"
    );
    assert!(
        plan.steps().is_empty(),
        "a refused step must not enter provenance"
    );
}

/// Budget exhaustion is a typed outcome, not a truncated result.
#[test]
fn budget_exhaustion_is_explicit() {
    let mut plan = Plan::new(options().with_memory_budget(1_024));
    let error = plan
        .admit(
            step(Determinism::NumericallyBounded),
            ScratchRequirement::PerElement {
                bytes_per_element: 64,
            },
            1_000,
        )
        .expect_err("64 bytes x 1000 elements exceeds a 1 KiB budget");

    assert!(matches!(error, GeomError::BudgetExceeded { .. }));
    assert!(plan.steps().is_empty());
}

/// An unbounded scratch requirement never fits a declared budget.
///
/// Admitting it would make the budget advisory, which is the failure the
/// budget exists to prevent.
#[test]
fn unbounded_scratch_never_fits_a_declared_budget() {
    let mut plan = Plan::new(options().with_memory_budget(usize::MAX));
    assert!(plan
        .admit(
            step(Determinism::NumericallyBounded),
            ScratchRequirement::Unbounded,
            1,
        )
        .is_err());
}

/// Provenance survives across operations, in order.
#[test]
fn provenance_accumulates_across_operations() {
    let mut plan = Plan::new(options());
    for operation in [
        Operation::ProfileTriangulation,
        Operation::Sweep,
        Operation::MeshBoolean,
    ] {
        plan.admit(
            PlanStep::new(
                operation,
                BackendId::new("test"),
                Determinism::NumericallyBounded,
                "unit cube",
            ),
            ScratchRequirement::None,
            0,
        )
        .expect("each step is admissible");
    }

    let recorded: Vec<Operation> = plan.steps().iter().map(|s| s.operation).collect();
    assert_eq!(
        recorded,
        vec![
            Operation::ProfileTriangulation,
            Operation::Sweep,
            Operation::MeshBoolean
        ],
        "provenance must preserve execution order"
    );
}

/// The recorded guarantee is the DELIVERED one, not the requested one.
///
/// A plan requesting `Topological` and running on a `Bitwise` backend must
/// record `Bitwise`. Recording the request instead would make provenance a
/// restatement of the caller's intent rather than evidence of what happened.
#[test]
fn provenance_records_what_was_delivered_not_what_was_asked() {
    let mut plan = Plan::new(options().with_determinism(Determinism::Topological));
    plan.admit(step(Determinism::Bitwise), ScratchRequirement::None, 0)
        .expect("stronger is admissible");

    assert_eq!(plan.options().determinism(), Determinism::Topological);
    assert_eq!(plan.steps()[0].guaranteed, Determinism::Bitwise);
}

/// Re-executing the same plan shape yields identical provenance.
#[test]
fn replaying_a_plan_reproduces_its_provenance() {
    let build = || {
        let mut plan = Plan::new(options());
        plan.admit(
            step(Determinism::NumericallyBounded),
            ScratchRequirement::None,
            0,
        )
        .expect("admissible");
        plan
    };
    assert_eq!(build(), build(), "the same plan must replay identically");
}
