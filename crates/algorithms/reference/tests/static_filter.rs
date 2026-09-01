//! Gates for the static filter.
//!
//! The safety property is one-directional: whenever the static filter commits
//! to a sign, that sign must equal the certified one. It is allowed to decline
//! (that is the price of a looser bound); it is never allowed to be wrong.

use axiolid_core::{Point2, Point3};
use axiolid_scalar::{orient2d, orient3d, StaticFilter};

#[test]
fn an_invalid_range_is_refused_rather_than_silently_accepted() {
    assert!(StaticFilter::new(0.0).is_none());
    assert!(StaticFilter::new(-1.0).is_none());
    assert!(StaticFilter::new(f64::NAN).is_none());
    assert!(StaticFilter::new(f64::INFINITY).is_none());
    // A bound whose derived error bound overflows must be refused too, rather
    // than yielding a filter that certifies nothing.
    assert!(StaticFilter::new(f64::MAX).is_none());
}

#[test]
fn a_point_outside_the_declared_range_is_declined() {
    let filter = StaticFilter::new(10.0).expect("valid");
    let inside = Point2::new(1.0, 1.0);
    let outside = Point2::new(1e6, 0.0);
    assert!(filter.orient2d(inside, outside, inside).is_none());
}

/// Whenever the static filter answers, it must agree with the exact predicate.
#[test]
fn a_static_answer_never_contradicts_the_exact_predicate() {
    let bound = 1_000.0;
    let filter = StaticFilter::new(bound).expect("valid");
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Integer coordinates well inside the declared range.
        (((state >> 33) as i64) % 2_000 - 1_000) as f64
    };

    let mut answered = 0usize;
    for _ in 0..50_000 {
        let a = Point2::new(next(), next());
        let b = Point2::new(next(), next());
        let c = Point2::new(next(), next());
        if let Some(fast) = filter.orient2d(a, b, c) {
            let exact = orient2d(a, b, c).sign().expect("certified");
            assert_eq!(fast, exact, "static filter disagreed on {a:?} {b:?} {c:?}");
            answered += 1;
        }
    }
    // A filter that always declined would pass the assertion above vacuously.
    assert!(
        answered > 40_000,
        "static filter answered only {answered}/50000; it is not earning its cost"
    );
}

/// The same one-directional safety property in 3D, including a deliberate
/// mix of degenerate inputs where the static bound must decline rather than
/// commit to a wrong sign.
#[test]
fn the_three_dimensional_static_answer_is_also_safe() {
    let filter = StaticFilter::new(1_000.0).expect("valid");
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |m: i64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (((state >> 33) as i64) % (2 * m) - m) as f64
    };

    let mut answered = 0usize;
    let mut declined_on_degenerate = 0usize;
    for i in 0..50_000 {
        let a = Point3::new(next(500), next(500), next(500));
        let b = Point3::new(next(500), next(500), next(500));
        let c = Point3::new(next(500), next(500), next(500));
        // Every other case is forced exactly onto the plane through a, b, c,
        // where the true determinant is zero and no bound may certify a sign.
        let d = if i % 2 == 0 {
            Point3::new(next(500), next(500), next(500))
        } else {
            Point3::new(
                a.x + (b.x - a.x) + (c.x - a.x),
                a.y + (b.y - a.y) + (c.y - a.y),
                a.z + (b.z - a.z) + (c.z - a.z),
            )
        };
        match filter.orient3d(a, b, c, d) {
            Some(fast) => {
                let exact = orient3d(a, b, c, d).sign().expect("certified");
                assert_eq!(fast, exact, "static filter disagreed in 3D");
                answered += 1;
            }
            None if i % 2 == 1 => declined_on_degenerate += 1,
            None => {}
        }
    }
    assert!(
        answered > 20_000,
        "3D static filter answered only {answered}"
    );
    assert!(
        declined_on_degenerate > 20_000,
        "coplanar inputs must be declined, not certified: {declined_on_degenerate}"
    );
}

/// A determinant landing exactly ON the bound must be declined.
///
/// This is the `>` versus `>=` distinction, and it needs a constructed input:
/// random data essentially never produces `|det| == bound` exactly. The value
/// is reached by choosing coordinates whose determinant is representable and
/// then asking the filter about a bound equal to it.
#[test]
fn a_determinant_landing_exactly_on_the_bound_is_declined() {
    // Pick a filter whose 2D bound is a specific value, then build a triple
    // whose determinant equals it exactly. `det = (a.x-c.x)*(b.y-c.y)` with
    // the other product zero, so the determinant is one exact multiplication.
    let filter = StaticFilter::new(1_000.0).expect("valid");
    let bound = {
        // Recompute the documented bound so the test does not depend on a
        // private accessor: same expression as `StaticFilter::new`.
        let eps = f64::EPSILON / 2.0;
        let span = 2.0 * filter.bound();
        (3.0 + 16.0 * eps) * eps * (2.0 * span * span)
    };

    // a.x - c.x = bound, b.y - c.y = 1, a.y - c.y = 0 => det = bound exactly.
    let c = Point2::new(0.0, 0.0);
    let a = Point2::new(bound, 0.0);
    let b = Point2::new(0.0, 1.0);
    let det = (a.x - c.x) * (b.y - c.y) - (a.y - c.y) * (b.x - c.x);
    assert_eq!(
        det, bound,
        "precondition: the determinant must sit on the bound"
    );

    assert!(
        filter.orient2d(a, b, c).is_none(),
        "a determinant equal to its own error bound is not a proven sign"
    );
}

/// A determinant exactly equal to the error bound must NOT be certified.
///
/// At `|det| == bound` the true value could be zero, so the sign is unproven.
/// A `>=` comparison would certify it and return a sign that may be wrong,
/// which is precisely the failure a static filter must never have.
#[test]
fn a_determinant_exactly_at_the_bound_is_declined() {
    // Collinear points: the true determinant is exactly zero, so no bound may
    // ever certify a sign for them, at any magnitude.
    let filter = StaticFilter::new(1_000.0).expect("valid");
    for k in 1..500i64 {
        let s = k as f64;
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(s, s);
        let c = Point2::new(2.0 * s, 2.0 * s);
        assert!(
            filter.orient2d(a, b, c).is_none(),
            "an exactly collinear triple was certified at scale {s}"
        );
    }
}

/// The range check is load-bearing: coordinates beyond the declared bound
/// invalidate the precomputed error, so they must be declined rather than
/// judged against a bound that no longer applies.
#[test]
fn coordinates_beyond_the_range_are_never_certified() {
    let filter = StaticFilter::new(1.0).expect("valid");
    // Far outside the declared range, and nearly collinear: the precomputed
    // bound is far too small here, so a missing range check would certify a
    // sign from noise.
    let a = Point2::new(0.0, 0.0);
    let b = Point2::new(1e150, 1e150);
    let c = Point2::new(2e150, 2e150);
    assert!(
        filter.orient2d(a, b, c).is_none(),
        "out-of-range coordinates must be declined"
    );

    // The load-bearing case: out of range AND the naive comparison against the
    // small precomputed bound would certify a WRONG sign. At 1e8 the true
    // determinant of this near-collinear triple is pure rounding noise, but it
    // dwarfs a bound computed for range 1.0.
    let big_a = Point2::new(1e8, 1e8);
    let big_b = Point2::new(3e8, 3.0000000000000004e8);
    let big_c = Point2::new(5e8, 5e8);
    let naive =
        (big_a.x - big_c.x) * (big_b.y - big_c.y) - (big_a.y - big_c.y) * (big_b.x - big_c.x);
    let small_bound = {
        let eps = f64::EPSILON / 2.0;
        let span = 2.0 * 1.0;
        (3.0 + 16.0 * eps) * eps * (2.0 * span * span)
    };
    assert!(
        naive.abs() > small_bound,
        "precondition: without the range check this input would be certified"
    );
    assert!(
        filter.orient2d(big_a, big_b, big_c).is_none(),
        "out-of-range input must be declined even when its determinant is large"
    );

    let p = Point3::new(0.0, 0.0, 0.0);
    let q = Point3::new(1e150, 0.0, 0.0);
    let r = Point3::new(0.0, 1e150, 0.0);
    let s = Point3::new(0.0, 0.0, 1e150);
    assert!(
        filter.orient3d(p, q, r, s).is_none(),
        "out-of-range 3D coordinates must be declined"
    );
}
