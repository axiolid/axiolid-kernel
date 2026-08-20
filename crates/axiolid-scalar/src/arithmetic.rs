//! Arbitrary-length expansion arithmetic.
//!
//! An *expansion* is a list of non-overlapping f64 components whose exact sum
//! is the value it represents. Two f64s can hold a product exactly; a list can
//! hold an arbitrary determinant exactly. This is what lets `orient3d`,
//! `incircle`, and `insphere` certify a sign rather than guess one.
//!
//! Components are ordered smallest to largest, so the sign of a non-zero
//! expansion is the sign of its last component.
//!
//! # Cost and where it is paid
//!
//! These operations allocate. That is deliberate and confined to the *exact*
//! path, which a filtered predicate reaches only when the floating-point
//! determinant is too close to zero to trust. The benchmark harness measures
//! the escalation rate precisely so this cost is a number, not a hope.

use axiolid_kernel::Sign;

use crate::expansion::{two_product, two_sum};

/// Sum of `a` and `b` where `|a| >= |b|` is already known.
///
/// Cheaper than [`two_sum`] by two operations. The precondition is not checked
/// in release builds; violating it silently produces a non-expansion, so every
/// caller here derives the ordering structurally rather than assuming it.
#[inline]
#[must_use]
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let b_virtual = sum - a;
    (sum, b - b_virtual)
}

/// Grow an expansion by one scalar: `e + b`, exactly.
///
/// Sweeps `b` through the components from smallest to largest, carrying the
/// rounding error forward. Zero components are dropped: they carry no value
/// and would break the non-overlapping invariant later.
#[must_use]
pub fn grow_expansion(e: &[f64], b: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(e.len() + 1);
    let mut carry = b;
    for &component in e {
        let (sum, error) = two_sum(carry, component);
        if error != 0.0 {
            out.push(error);
        }
        carry = sum;
    }
    if carry != 0.0 || out.is_empty() {
        out.push(carry);
    }
    out
}

/// Sum of two expansions, exactly.
///
/// Merges the two component lists in magnitude order, then runs a single
/// carry-propagating pass. Both inputs must be non-overlapping expansions in
/// increasing magnitude order; the result is the same.
#[must_use]
pub fn expansion_sum(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut result = a.to_vec();
    for &component in b {
        result = grow_expansion(&result, component);
    }
    result
}

/// Scale an expansion by a scalar, exactly.
///
/// Each component contributes a two-term product, so the result has at most
/// twice as many components as the input.
#[must_use]
pub fn scale_expansion(e: &[f64], b: f64) -> Vec<f64> {
    let mut out = Vec::with_capacity(e.len() * 2);
    let mut carry = 0.0;
    for &component in e {
        let (product, product_error) = two_product(component, b);
        let (sum, error) = two_sum(carry, product_error);
        if error != 0.0 {
            out.push(error);
        }
        let (new_carry, hi_error) = fast_two_sum(product, sum);
        if hi_error != 0.0 {
            out.push(hi_error);
        }
        carry = new_carry;
    }
    if carry != 0.0 || out.is_empty() {
        out.push(carry);
    }
    out
}

/// Negate every component. Exact: negation is always representable.
#[must_use]
pub fn negate_expansion(e: &[f64]) -> Vec<f64> {
    e.iter().map(|c| -c).collect()
}

/// Sign of an expansion.
///
/// The components are non-overlapping and ordered by increasing magnitude, so
/// the largest non-zero component dominates the sum and decides the sign.
#[must_use]
pub fn expansion_sign(e: &[f64]) -> Sign {
    for &component in e.iter().rev() {
        if component > 0.0 {
            return Sign::Positive;
        }
        if component < 0.0 {
            return Sign::Negative;
        }
    }
    Sign::Zero
}
