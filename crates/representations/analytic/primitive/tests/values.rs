//! Contracts for exact primitive solids and half-spaces.
//!
//! Values here stay exact until an explicit tessellation step, so what this
//! crate owes consumers is a faithful, un-normalised representation plus the
//! validation on `ClipMargin`.

use axiolid_core::{Plane3, Point3, Vec3};
use axiolid_primitive::{ClipMargin, HalfSpace};

fn plane() -> Plane3 {
    Plane3 {
        origin: Point3::ZERO,
        normal: Vec3::Z,
    }
}

/// A clip margin must be finite and strictly positive.
///
/// The margin expands a subject's bounds so an unbounded half-space can take
/// part in a finite mesh operation. Zero would leave the boundary exactly on
/// the bound, where the clip is ill-defined; negative would shrink it.
#[test]
fn clip_margin_refuses_non_positive_and_non_finite_factors() {
    for bad in [0.0, -0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            ClipMargin::new(bad).is_none(),
            "clip margin {bad} must be refused"
        );
    }
}

/// A valid factor round-trips unchanged.
#[test]
fn clip_margin_preserves_its_factor() {
    let margin = ClipMargin::new(1.25).expect("positive finite factor");
    assert_eq!(margin.factor(), 1.25);
}

/// `agreement` selects the side, and both sides are representable.
///
/// The flag is the only thing distinguishing the two half-spaces of a plane, so
/// consumers must branch on it rather than inferring a side from the normal.
#[test]
fn both_sides_of_a_plane_are_representable() {
    let normal_side = HalfSpace {
        boundary: plane(),
        agreement: true,
    };
    let opposite_side = HalfSpace {
        boundary: plane(),
        agreement: false,
    };
    assert_ne!(
        normal_side, opposite_side,
        "the agreement flag distinguishes the two half-spaces of one plane"
    );
    assert_eq!(
        normal_side.boundary, opposite_side.boundary,
        "both share the same boundary plane"
    );
}

/// The boundary plane is stored verbatim, including a non-unit normal.
///
/// Normalisation is a consumer's decision, and silently rewriting the value
/// here would hide malformed input from the code positioned to reject it.
#[test]
fn boundary_planes_are_stored_verbatim() {
    let unnormalised = Plane3 {
        origin: Point3::new(1.0, 2.0, 3.0),
        normal: Vec3::Z * 4.0,
    };
    let half_space = HalfSpace {
        boundary: unnormalised,
        agreement: true,
    };
    assert_eq!(
        half_space.boundary.normal,
        Vec3::Z * 4.0,
        "a non-unit normal survives construction unmodified"
    );
    assert_eq!(half_space.boundary.origin, Point3::new(1.0, 2.0, 3.0));
}
