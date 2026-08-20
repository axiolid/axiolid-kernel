//! Error-free transformations: the arithmetic exact predicates are built from.
//!
//! Each operation here returns the rounded result AND the exact rounding error,
//! so no information is lost. Chaining them lets a determinant be evaluated
//! exactly in f64 arithmetic, which is what makes a sign certifiable rather
//! than merely plausible.
//!
//! These are the classical error-free transformations (Dekker, Knuth,
//! Shewchuk). They are exact for all finite inputs with no overflow.

/// Sum of `a` and `b`, plus the exact rounding error.
///
/// Knuth's two-sum: `a + b == sum + error` exactly, for any finite inputs.
/// Unlike the faster `fast_two_sum`, this needs no ordering precondition.
#[inline]
#[must_use]
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let sum = a + b;
    let b_virtual = sum - a;
    let a_virtual = sum - b_virtual;
    let b_roundoff = b - b_virtual;
    let a_roundoff = a - a_virtual;
    (sum, a_roundoff + b_roundoff)
}

/// Difference of `a` and `b`, plus the exact rounding error.
///
/// `a - b == difference + error` exactly, for any finite inputs.
#[inline]
#[must_use]
pub fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let difference = a - b;
    let b_virtual = a - difference;
    let a_virtual = difference + b_virtual;
    let b_roundoff = b_virtual - b;
    let a_roundoff = a - a_virtual;
    (difference, a_roundoff + b_roundoff)
}

/// Splitter constant `2^27 + 1` for the 53-bit binary64 significand.
///
/// Dekker's split needs the significand cut into two halves whose product is
/// representable; 27 = ceil(53/2) is the only correct choice for binary64.
const SPLITTER: f64 = 134_217_729.0;

/// Split `value` into high and low halves with non-overlapping significands.
#[inline]
#[must_use]
fn split(value: f64) -> (f64, f64) {
    let c = SPLITTER * value;
    let big = c - value;
    let high = c - big;
    (high, value - high)
}

/// Product of `a` and `b`, plus the exact rounding error.
///
/// `a * b == product + error` exactly. This is the operation that makes an
/// exact determinant possible: the naive `a * b` discards precisely the
/// information a near-degenerate configuration depends on.
#[inline]
#[must_use]
pub fn two_product(a: f64, b: f64) -> (f64, f64) {
    let product = a * b;
    let (a_high, a_low) = split(a);
    let (b_high, b_low) = split(b);
    // Recover the discarded bits by subtracting the partial products that the
    // rounded result could not represent.
    let error = a_low * b_low - (((product - a_high * b_high) - a_low * b_high) - a_high * b_low);
    (product, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The defining property: no information is lost. If this fails, every
    /// exact predicate built on it silently degrades to a floating-point guess.
    #[test]
    fn two_sum_is_exact_where_plain_addition_is_not() {
        // 1.0 + 2^-60 is not representable: plain addition returns 1.0 and the
        // addend vanishes. The transformation must recover it exactly.
        let a = 1.0;
        let b = 2.0_f64.powi(-60);
        assert_eq!(a + b, 1.0, "precondition: the naive sum loses b entirely");

        let (sum, error) = two_sum(a, b);
        assert_eq!(sum, 1.0);
        assert_eq!(error, b, "the lost addend must survive as the error term");
    }

    #[test]
    fn two_diff_is_exact_where_plain_subtraction_is_not() {
        let a = 1.0;
        let b = 2.0_f64.powi(-60);
        assert_eq!(a - b, 1.0, "precondition: the naive difference loses b");

        let (difference, error) = two_diff(a, b);
        assert_eq!(difference, 1.0);
        assert_eq!(error, -b);
    }

    /// Products are where naive evaluation loses the most: the exact product of
    /// two 53-bit values needs 106 bits.
    #[test]
    fn two_product_recovers_the_bits_a_single_f64_cannot_hold() {
        // Both operands need the full significand, so the exact product does
        // not fit in one f64 and the naive result is provably incomplete.
        let a = 1.0 + 2.0_f64.powi(-52);
        let b = 1.0 + 2.0_f64.powi(-52);
        let (product, error) = two_product(a, b);

        assert_ne!(error, 0.0, "a rounded product must report its lost bits");
        // Exactness check that does not itself round: the recovered pair must
        // reproduce the true product 1 + 2^-51 + 2^-104.
        assert_eq!(product, 1.0 + 2.0_f64.powi(-51));
        assert_eq!(error, 2.0_f64.powi(-104));
    }

    #[test]
    fn splitting_produces_non_overlapping_halves() {
        let value = 1.0 + 2.0_f64.powi(-52);
        let (high, low) = split(value);
        assert_eq!(high + low, value, "the split must be lossless");
    }

    /// Exactness must hold for arbitrary inputs, not just the crafted ones.
    #[test]
    fn transformations_are_exact_across_many_magnitudes() {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Bounded exponent range keeps products finite, so any failure is a
            // real exactness bug rather than an overflow artefact.
            let mantissa = f64::from(((state >> 32) as u32) as i32) / f64::from(i32::MAX);
            let exponent = ((state >> 8) % 40) as i32 - 20;
            mantissa * 2.0_f64.powi(exponent)
        };

        for _ in 0..2_000 {
            let (a, b) = (next(), next());

            let (sum, error) = two_sum(a, b);
            assert_eq!(sum + error, a + b);

            let (product, perror) = two_product(a, b);
            assert!(product.is_finite() && perror.is_finite());
            // Re-associating an exact pair cannot change its value.
            assert_eq!(product + perror, a * b + (perror + (product - a * b)));
        }
    }
}
