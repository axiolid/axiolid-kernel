//! Contract gates for certified signs and the escalation ladder.

use axiolid_kernel::{Certified, EscalationLadder, Precision, Sign};

/// A value inside its own error bound must not yield a sign. This is the whole
/// point: an uncertified float can never reach a topology decision.
#[test]
fn a_value_within_its_error_bound_is_uncertain_not_zero() {
    let result = Certified::from_filter(1e-18, 1e-15, Precision::F64);
    assert_eq!(result.sign(), None, "must not claim a sign");
    assert!(!result.is_certain());
    assert_eq!(
        result,
        Certified::Uncertain {
            attempted: Precision::F64
        }
    );
}

/// A value outside the bound is decidable, and reports which tier proved it.
#[test]
fn a_value_outside_its_error_bound_is_certified_with_its_tier() {
    assert_eq!(
        Certified::from_filter(1.0, 1e-15, Precision::F32).sign(),
        Some(Sign::Positive)
    );
    assert_eq!(
        Certified::from_filter(-1.0, 1e-15, Precision::F32).sign(),
        Some(Sign::Negative)
    );
    assert_eq!(
        Certified::from_filter(1.0, 1e-15, Precision::F32),
        Certified::Certain {
            sign: Sign::Positive,
            precision: Precision::F32
        }
    );
}

/// Exactly on the bound is NOT decidable. The bound is inclusive because an
/// error of exactly `bound` is permitted, so the true value may be zero.
#[test]
fn a_value_exactly_on_its_error_bound_is_uncertain() {
    assert_eq!(
        Certified::from_filter(1e-15, 1e-15, Precision::F64).sign(),
        None
    );
    assert_eq!(
        Certified::from_filter(-1e-15, 1e-15, Precision::F64).sign(),
        None
    );
}

/// An exact zero is a *certain* degenerate answer, not a failure to decide.
/// Conflating the two would make degeneracy indistinguishable from ambiguity.
#[test]
fn exact_zero_is_certain_and_distinct_from_uncertain() {
    let exact_zero = Certified::exact(0);
    assert_eq!(exact_zero.sign(), Some(Sign::Zero));
    assert!(exact_zero.is_certain());

    let ambiguous = Certified::from_filter(0.0, 1e-15, Precision::F64);
    assert_eq!(ambiguous.sign(), None);
    assert_ne!(exact_zero, ambiguous);
}

/// Non-finite inputs are never certifiable, however large.
#[test]
fn non_finite_values_are_never_certified() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Certified::from_filter(value, 1e-15, Precision::F64).sign(),
            None,
            "{value} must not certify"
        );
    }
    assert_eq!(
        Certified::from_filter(1.0, f64::NAN, Precision::F64).sign(),
        None
    );
    assert_eq!(
        Certified::from_filter(1.0, -1.0, Precision::F64).sign(),
        None
    );
}

/// The ladder steps f32 -> f64 -> exact, and stops at its ceiling.
#[test]
fn the_ladder_escalates_in_order_and_stops_at_its_ceiling() {
    let full = EscalationLadder::exact();
    assert_eq!(full.next_after(Precision::F32), Some(Precision::F64));
    assert_eq!(full.next_after(Precision::F64), Some(Precision::Exact));
    assert_eq!(full.next_after(Precision::Exact), None);

    let capped = EscalationLadder::new(Precision::F64);
    assert_eq!(capped.next_after(Precision::F32), Some(Precision::F64));
    assert_eq!(
        capped.next_after(Precision::F64),
        None,
        "must not exceed ceiling"
    );
}

/// Only an exact ceiling guarantees every sign becomes decidable. A caller that
/// needs a guaranteed decision must be able to see this before dispatching.
#[test]
fn only_an_exact_ladder_is_total() {
    assert!(EscalationLadder::exact().is_total());
    assert!(!EscalationLadder::new(Precision::F64).is_total());
    assert!(!EscalationLadder::new(Precision::F32).is_total());
}

/// `Mixed` is a strategy, not a rung: it must never appear in an escalation
/// sequence, or a ladder could "escalate" sideways into an ambiguous tier.
#[test]
fn mixed_is_not_a_rung_of_the_ladder() {
    let full = EscalationLadder::exact();
    assert!(!full.permits(Precision::Mixed));
    assert_eq!(full.next_after(Precision::Mixed), None);
    assert!(!full.rungs().any(|rung| rung == Precision::Mixed));
}

/// The rungs a ladder will attempt, in weakest-first order.
#[test]
fn a_ladder_reports_the_tiers_it_will_attempt() {
    assert_eq!(
        EscalationLadder::exact().rungs().collect::<Vec<_>>(),
        vec![Precision::F32, Precision::F64, Precision::Exact]
    );
    assert_eq!(
        EscalationLadder::new(Precision::F64)
            .rungs()
            .collect::<Vec<_>>(),
        vec![Precision::F32, Precision::F64]
    );
}
