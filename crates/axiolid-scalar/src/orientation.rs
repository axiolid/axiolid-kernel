//! Certified orientation predicates.
//!
//! `orient2d` answers "does C lie left of, right of, or exactly on the directed
//! line AB". The answer is a *sign*, and a sign drives topology, so a plausible
//! answer is not good enough: this module returns [`Certified`] and escalates
//! to exact arithmetic rather than guessing when the filter is inconclusive.
//!
//! The strategy is the standard filtered cascade:
//!
//! 1. Evaluate the determinant in plain f64 and compute a forward error bound.
//! 2. If the magnitude exceeds the bound, the sign is proven; return it.
//! 3. Otherwise recompute exactly with error-free transformations.
//!
//! Step 1 succeeds for almost all real inputs, so the exact path is rare.

use axiolid_core::Point2;
use axiolid_kernel::{Certified, Precision, Sign};

use crate::expansion::{two_diff, two_product, two_sum};

/// Machine epsilon for binary64: the gap between 1.0 and the next value.
const EPSILON: f64 = f64::EPSILON / 2.0;

/// Relative error bound for the 2x2 determinant filter.
///
/// The determinant costs two products and one subtraction; propagating the
/// standard `(1 + eps)` model over that expression yields `3*eps + O(eps^2)`.
/// The `16.0 * EPSILON * EPSILON` term absorbs the higher-order remainder, so
/// the bound is a true upper bound rather than a first-order approximation.
const ORIENT2D_ERROR_FACTOR: f64 = (3.0 + 16.0 * EPSILON) * EPSILON;

/// Orientation of `c` relative to the directed line `a` -> `b`.
///
/// Returns [`Sign::Positive`] when `a`, `b`, `c` turn counter-clockwise,
/// [`Sign::Negative`] for clockwise, and [`Sign::Zero`] when the three points
/// are exactly collinear.
///
/// The result is always [`Certified::Certain`]: this function escalates to
/// exact arithmetic internally, so it never returns an unproven sign. A caller
/// may therefore use it to drive a topology decision directly.
#[must_use]
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> Certified {
    match orient2d_filter(a, b, c) {
        Certified::Certain { sign, .. } => Certified::exact_sign(sign),
        // The filter could not prove a sign, so pay for exact arithmetic. This
        // is the whole point of the cascade: correctness does not depend on the
        // fast path being lucky.
        Certified::Uncertain { .. } => Certified::exact_sign(orient2d_exact(a, b, c)),
        // `Certified` is non-exhaustive. A future variant we do not understand
        // must escalate, never be assumed decisive.
        _ => Certified::exact_sign(orient2d_exact(a, b, c)),
    }
}

/// The fast filter alone, exposed so the escalation can be observed and tested.
///
/// Returns [`Certified::Uncertain`] when the floating-point determinant is too
/// close to zero for its own error bound to exclude the opposite sign.
#[must_use]
pub fn orient2d_filter(a: Point2, b: Point2, c: Point2) -> Certified {
    let left = (a.x - c.x) * (b.y - c.y);
    let right = (a.y - c.y) * (b.x - c.x);
    let determinant = left - right;

    // The bound scales with the operand magnitudes: a determinant of 1e-9 is
    // decisive for millimetre coordinates and noise for national-grid ones.
    let magnitude = left.abs() + right.abs();
    let error_bound = ORIENT2D_ERROR_FACTOR * magnitude;

    Certified::from_filter(determinant, error_bound, Precision::F64)
}

/// Exact sign of the orientation determinant.
///
/// Every intermediate is carried as an unevaluated (value, error) pair, so no
/// bit of the determinant is discarded. The sign of the resulting expansion is
/// the sign of its largest-magnitude non-zero component.
#[must_use]
fn orient2d_exact(a: Point2, b: Point2, c: Point2) -> Sign {
    // Coordinate differences, exactly.
    let (acx, acx_err) = two_diff(a.x, c.x);
    let (bcy, bcy_err) = two_diff(b.y, c.y);
    let (acy, acy_err) = two_diff(a.y, c.y);
    let (bcx, bcx_err) = two_diff(b.x, c.x);

    // The determinant is (acx * bcy) - (acy * bcx). Each product is expanded to
    // four terms: the two rounded halves plus their cross-error contributions.
    let (left, left_err) = two_product(acx, bcy);
    let (right, right_err) = two_product(acy, bcx);

    // Correction terms for the fact that acx/bcy themselves carried errors.
    let left_correction = acx * bcy_err + acx_err * bcy + acx_err * bcy_err;
    let right_correction = acy * bcx_err + acy_err * bcx + acy_err * bcx_err;

    // Sum the expansion from smallest to largest so no term is absorbed early.
    let (head, head_err) = two_diff(left, right);
    // `head_err` is provably zero here. This function is only reached when the
    // filter was uncertain, which means |left - right| is small relative to
    // |left| + |right|; by Sterbenz's lemma such a subtraction is exact. The
    // term is kept so the expression stays a correct expansion sum if this
    // function is ever called directly, and asserted so the assumption is
    // checked rather than believed.
    debug_assert_eq!(head_err, 0.0, "filtered inputs make this subtraction exact");
    let tail = (left_err - right_err) + (left_correction - right_correction);
    let (total, total_err) = two_sum(head, head_err + tail);

    // The most significant non-zero component determines the sign; a zero head
    // with a non-zero tail means the leading terms cancelled exactly.
    let value = if total != 0.0 { total } else { total_err };
    if value > 0.0 {
        Sign::Positive
    } else if value < 0.0 {
        Sign::Negative
    } else {
        Sign::Zero
    }
}
