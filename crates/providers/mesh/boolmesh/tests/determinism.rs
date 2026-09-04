//! Determinism is declared honestly, and parallelism cannot weaken it (#80).
//!
//! Upstream `boolmesh` dedups vertices through a randomly-seeded `HashMap`,
//! so the general path's vertex ordering varies between processes even
//! single-threaded. The provider must therefore never claim
//! `Determinism::Bitwise`, at any thread count.
//!
//! These tests exist because the `parallel` feature is the kind of change
//! that silently breaks a reproducibility promise: enabling threads is a
//! one-line Cargo edit, and nothing else in the build would notice.

use axiolid_contracts::Determinism;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_boolean_contract::MeshBoolean;

/// The provider never claims bitwise reproducibility.
///
/// This must hold whether or not `parallel` is enabled, which is why the
/// assertion is unconditional rather than behind a `cfg`.
#[test]
fn the_general_path_never_claims_bitwise() {
    let provider = BoolmeshBoolean;
    assert_ne!(
        provider.determinism(),
        Determinism::Bitwise,
        "the general path dedups through a randomly-seeded HashMap upstream, \
         so its vertex ordering is not reproducible across processes"
    );
}

/// The declared level is exactly `Topological`, not merely "not Bitwise".
///
/// Pinning the value catches a well-meaning upgrade to `NumericallyBounded`
/// or `Bitwise` that has no evidence behind it.
#[test]
fn the_declared_level_is_topological() {
    assert_eq!(BoolmeshBoolean.determinism(), Determinism::Topological);
}

/// A `Bitwise` request is not satisfied by what this provider guarantees.
///
/// This is the admission check `Plan::admit` performs. Asserting it here
/// means the refusal is pinned at the provider, not only in the plan crate:
/// if someone raised the declared level, this fails even though the plan
/// logic itself is untouched.
#[test]
fn a_bitwise_request_is_not_satisfied() {
    let guaranteed = BoolmeshBoolean.determinism();
    assert!(
        !guaranteed.satisfies(Determinism::Bitwise),
        "a caller asking for bitwise reproducibility must be refused, \
         not silently handed a schedule-dependent result"
    );
    // The weaker request the provider CAN honour, so the test proves the
    // refusal is specific rather than a blanket rejection.
    assert!(guaranteed.satisfies(Determinism::Topological));
}
