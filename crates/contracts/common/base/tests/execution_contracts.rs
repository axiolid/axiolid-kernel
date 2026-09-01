//! Contract gates for the execution policy surface.

use axiolid_contracts::{
    BackendId, DataResidency, Determinism, ExecutionOptions, GeomError, Residency,
    ScratchRequirement,
};
use axiolid_core::Tolerance;

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

// --- R9: determinism is three contracts, not one flag -------------------

#[test]
fn determinism_levels_form_a_strength_lattice() {
    use Determinism::{BestEffort, Bitwise, NumericallyBounded, Topological};

    // Every level satisfies itself and everything weaker.
    for (stronger, weaker) in [
        (Topological, BestEffort),
        (NumericallyBounded, Topological),
        (Bitwise, NumericallyBounded),
        (Bitwise, BestEffort),
    ] {
        assert!(
            stronger.satisfies(weaker),
            "{stronger:?} must satisfy {weaker:?}"
        );
        assert!(
            !weaker.satisfies(stronger),
            "{weaker:?} must NOT satisfy {stronger:?}"
        );
    }
    for level in [BestEffort, Topological, NumericallyBounded, Bitwise] {
        assert!(level.satisfies(level), "{level:?} must satisfy itself");
    }
}

/// The whole point of splitting the enum: stable ordering and bitwise identity
/// are different promises. A backend offering only the former must be rejected
/// for the latter, which a single `Required` flag could not express.
#[test]
fn topological_determinism_does_not_promise_bitwise_identity() {
    assert!(!Determinism::Topological.satisfies(Determinism::Bitwise));
    assert!(!Determinism::NumericallyBounded.satisfies(Determinism::Bitwise));
}

/// Each of the three named contracts must be independently selectable and
/// survive a round trip through the builder.
#[test]
fn every_determinism_contract_is_distinctly_selectable() {
    let levels = [
        Determinism::BestEffort,
        Determinism::Topological,
        Determinism::NumericallyBounded,
        Determinism::Bitwise,
    ];
    for level in levels {
        assert_eq!(
            options().with_determinism(level).determinism(),
            level,
            "{level:?} must round-trip unchanged"
        );
    }
    // Distinct variants, not aliases of one another.
    for (i, a) in levels.iter().enumerate() {
        for (j, b) in levels.iter().enumerate() {
            assert_eq!(i == j, a == b, "{a:?} vs {b:?} must be distinct");
        }
    }
}

#[test]
fn the_default_policy_is_numerically_bounded_not_best_effort() {
    // A geometry kernel defaulting to BestEffort would silently permit
    // unstable topology; defaulting to Bitwise would ban every parallel
    // reduction. NumericallyBounded is the honest middle.
    assert_eq!(options().determinism(), Determinism::NumericallyBounded);
}

// --- R8: residency is part of the execution plan ------------------------

#[test]
fn residency_defaults_to_host_and_is_explicitly_overridable() {
    assert_eq!(options().residency(), DataResidency::HOST);

    let gpu = BackendId::new("test-gpu");
    let plan = DataResidency::new(Residency::Device(gpu), Residency::Host);
    let configured = options().with_residency(plan);

    assert_eq!(configured.residency().input(), Residency::Device(gpu));
    assert_eq!(configured.residency().output(), Residency::Host);
}

#[test]
fn residency_distinguishes_device_local_from_host_readable() {
    let gpu = BackendId::new("gpu-a");
    let other = BackendId::new("gpu-b");

    assert!(Residency::Device(gpu).is_local_to(gpu));
    assert!(!Residency::Device(gpu).is_local_to(other));
    assert!(!Residency::Device(gpu).is_host_readable());

    // Unified memory is readable from both sides without a copy.
    assert!(Residency::Unified(gpu).is_local_to(gpu));
    assert!(Residency::Unified(gpu).is_host_readable());

    // Host data is host-readable but never device-local: reaching it from a
    // device costs a transfer, which is exactly what routing must see.
    assert!(Residency::Host.is_host_readable());
    assert!(!Residency::Host.is_local_to(gpu));
}

#[test]
fn a_transfer_free_plan_requires_both_ends_on_the_device() {
    let gpu = BackendId::new("gpu-a");
    let resident = DataResidency::new(Residency::Device(gpu), Residency::Device(gpu));
    assert!(resident.is_transfer_free_on(gpu));

    // Device-resident input but host-wanted output still costs a readback.
    let readback = DataResidency::new(Residency::Device(gpu), Residency::Host);
    assert!(!readback.is_transfer_free_on(gpu));
    assert!(!DataResidency::HOST.is_transfer_free_on(gpu));
}

// --- R4: scratch is declared and the budget is enforced -----------------

#[test]
fn scratch_requirements_report_bounds() {
    assert_eq!(ScratchRequirement::None.upper_bound_bytes(1_000), Some(0));
    assert_eq!(
        ScratchRequirement::Fixed { bytes: 4_096 }.upper_bound_bytes(1_000),
        Some(4_096)
    );
    assert_eq!(
        ScratchRequirement::PerElement {
            bytes_per_element: 32
        }
        .upper_bound_bytes(100),
        Some(3_200)
    );
    assert_eq!(
        ScratchRequirement::Unbounded.upper_bound_bytes(1),
        None,
        "unbounded scratch must not report a bound"
    );
}

/// A per-element bound must not silently wrap into a small number on overflow;
/// that would turn an enormous requirement into an apparently affordable one.
#[test]
fn per_element_bounds_saturate_to_unbounded_on_overflow() {
    let huge = ScratchRequirement::PerElement {
        bytes_per_element: usize::MAX,
    };
    assert_eq!(huge.upper_bound_bytes(2), None);
    assert!(!huge.fits_budget(&options().with_memory_budget(1_024), 2));
}

#[test]
fn an_unbounded_requirement_never_fits_a_declared_budget() {
    let budgeted = options().with_memory_budget(1_000_000);
    assert!(!ScratchRequirement::Unbounded.fits_budget(&budgeted, 1));

    // With no budget declared, anything is permitted.
    assert!(ScratchRequirement::Unbounded.fits_budget(&options(), 1));
}

#[test]
fn budgets_admit_what_fits_and_reject_what_does_not() {
    let budgeted = options().with_memory_budget(1_000);
    let per_element = ScratchRequirement::PerElement {
        bytes_per_element: 10,
    };

    assert!(per_element.fits_budget(&budgeted, 100), "1000 <= 1000");
    assert!(!per_element.fits_budget(&budgeted, 101), "1010 > 1000");
}

#[test]
fn charging_scratch_reports_a_structured_budget_error() {
    let budgeted = options().with_memory_budget(512);
    assert!(budgeted.charge_scratch(512).is_ok());
    assert!(matches!(
        budgeted.charge_scratch(513),
        Err(GeomError::BudgetExceeded { resource: "memory" })
    ));
    // No budget means no ceiling.
    assert!(options().charge_scratch(usize::MAX).is_ok());
}
