//! Gates for arbitrary-length expansion arithmetic.
//!
//! Every predicate's exact path rests on these, so they are checked against an
//! independent exact oracle rather than against each other. The oracle is
//! rational arithmetic over i128, exact for the bounded magnitudes used here.

use axiolid_contracts::Sign;
use axiolid_reference::arithmetic::{
    expansion_sign, expansion_sum, grow_expansion, negate_expansion, scale_expansion,
};

/// Exact value of an expansion as a rational, for comparison.
///
/// Every f64 with a bounded exponent is exactly `mantissa * 2^exponent`; summing
/// those as scaled integers is exact when the shifts stay inside i128.
fn exact_sum_scaled(e: &[f64], scale_bits: u32) -> i128 {
    e.iter()
        .map(|&c| {
            let scaled = c * 2f64.powi(scale_bits as i32);
            assert_eq!(
                scaled.fract(),
                0.0,
                "component {c} is not exact at this scale"
            );
            scaled as i128
        })
        .sum()
}

/// Deterministic values on a dyadic grid, so the oracle stays exact.
fn grid_values(seed: u64, count: usize, scale_bits: u32) -> Vec<f64> {
    let mut state = seed | 1;
    let scale = 2f64.powi(-(scale_bits as i32));
    (0..count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Small integers over a fixed dyadic scale: representable exactly,
            // and their products and sums stay inside i128.
            let integer = ((state >> 40) as i64 % 2_000) - 1_000;
            (integer as f64) * scale
        })
        .collect()
}

const SCALE: u32 = 20;

/// Growing an expansion must preserve its exact value.
#[test]
fn growing_preserves_the_exact_sum() {
    for seed in 1..200u64 {
        let values = grid_values(seed, 6, SCALE);
        let mut expansion: Vec<f64> = vec![0.0];
        let mut expected: i128 = 0;
        for &v in &values {
            expansion = grow_expansion(&expansion, v);
            expected += (v * 2f64.powi(SCALE as i32)) as i128;
            assert_eq!(
                exact_sum_scaled(&expansion, SCALE),
                expected,
                "seed {seed}: expansion value drifted"
            );
        }
    }
}

/// Summing two expansions must equal summing their values.
#[test]
fn expansion_sum_matches_the_exact_oracle() {
    for seed in 1..200u64 {
        let left = grid_values(seed, 4, SCALE);
        let right = grid_values(seed.wrapping_mul(7) | 1, 4, SCALE);

        let a = left.iter().fold(vec![0.0], |e, &v| grow_expansion(&e, v));
        let b = right.iter().fold(vec![0.0], |e, &v| grow_expansion(&e, v));
        let total = expansion_sum(&a, &b);

        assert_eq!(
            exact_sum_scaled(&total, SCALE),
            exact_sum_scaled(&a, SCALE) + exact_sum_scaled(&b, SCALE),
            "seed {seed}"
        );
    }
}

/// Scaling must multiply the value exactly.
#[test]
fn scaling_multiplies_the_exact_value() {
    for seed in 1..200u64 {
        let values = grid_values(seed, 4, SCALE);
        let e = values
            .iter()
            .fold(vec![0.0], |acc, &v| grow_expansion(&acc, v));
        // A small power of two keeps the product exactly representable, so any
        // mismatch is an arithmetic bug rather than an oracle limitation.
        let factor = 4.0;
        let scaled = scale_expansion(&e, factor);
        assert_eq!(
            exact_sum_scaled(&scaled, SCALE),
            exact_sum_scaled(&e, SCALE) * 4,
            "seed {seed}"
        );
    }
}

/// The sign must agree with the sign of the exact value, including zero.
#[test]
fn the_sign_matches_the_exact_value_including_exact_zero() {
    for seed in 1..300u64 {
        let values = grid_values(seed, 5, SCALE);
        let e = values
            .iter()
            .fold(vec![0.0], |acc, &v| grow_expansion(&acc, v));
        let exact = exact_sum_scaled(&e, SCALE);
        let expected = match exact.signum() {
            1 => Sign::Positive,
            -1 => Sign::Negative,
            _ => Sign::Zero,
        };
        assert_eq!(expansion_sign(&e), expected, "seed {seed}: value {exact}");

        // Cancelling an expansion against its own negation must yield exact
        // zero: this is the case a sloppy sign function reports wrongly.
        let cancelled = expansion_sum(&e, &negate_expansion(&e));
        assert_eq!(expansion_sign(&cancelled), Sign::Zero, "seed {seed}");
    }
}

/// The sign must come from the LARGEST component, not the smallest.
///
/// Components are ordered smallest-to-largest and are non-overlapping, so the
/// last non-zero one dominates the sum. Reading from the front returns the
/// sign of a tiny correction term, which is frequently opposite. A mutation
/// reversing the iteration order survived every other test in this file
/// because those expansions happened to be single-component.
#[test]
fn the_sign_comes_from_the_dominant_component() {
    // A genuine two-component expansion whose parts disagree in sign:
    // 1.0 - 2^-60 is represented as [-2^-60, 1.0]. The value is positive.
    let e = grow_expansion(&[1.0], -(2f64.powi(-60)));
    assert!(
        e.len() >= 2,
        "precondition: this must be a multi-component expansion, got {e:?}"
    );
    assert!(
        e[0] < 0.0 && e[e.len() - 1] > 0.0,
        "precondition: the components must disagree in sign, got {e:?}"
    );
    assert_eq!(
        expansion_sign(&e),
        Sign::Positive,
        "the dominant component decides the sign"
    );

    // And the mirror image, so the test cannot pass by always answering
    // Positive.
    let f = negate_expansion(&e);
    assert_eq!(expansion_sign(&f), Sign::Negative);
}
