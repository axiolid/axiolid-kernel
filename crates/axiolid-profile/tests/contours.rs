//! Contracts for contour and centre-line profile values.
//!
//! Parameterized section variants are covered by inline tests in
//! `src/parameterized.rs`. What is pinned here is the part with no coverage:
//! `Contour` assembly and the centre-line width convention.

use axiolid_core::{Frame2, Interval, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_profile::{CenterLineProfile, Contour, ContourProfile, ProfileSegment};

fn segment(curve: Curve2) -> ProfileSegment {
    ProfileSegment {
        curve,
        domain: Interval {
            start: 0.0,
            end: 1.0,
        },
        same_sense: true,
    }
}

fn line() -> Curve2 {
    Curve2::Line(Line2 {
        origin: Point2::ZERO,
        direction: Vec2::X,
    })
}

/// A centre line stores half the width it is constructed from.
///
/// The field is deliberately a half-width so both offset sides are symmetric
/// by construction. This pins the factor of two at the boundary, which is
/// exactly where a silent halving error would produce plausible-looking but
/// wrong geometry.
#[test]
fn a_centre_line_halves_the_width_it_is_given() {
    let profile = CenterLineProfile::from_width(Contour::new(vec![segment(line())]), 0.4);
    assert_eq!(profile.half_width, 0.2, "the stored value is a half-width");
    assert_eq!(profile.width(), 0.4, "the full width round-trips");
}

/// Width round-trips through the accessor for a range of magnitudes.
#[test]
fn centre_line_width_round_trips() {
    for width in [1.0e-6, 0.25, 1.0, 1000.0] {
        let profile = CenterLineProfile::from_width(Contour::default(), width);
        assert_eq!(
            profile.width(),
            width,
            "width {width} must survive the half-width round trip"
        );
    }
}

/// `Contour::new` stores segments verbatim without repairing gaps.
///
/// Two segments that do not meet stay un-repaired: closure is validated
/// elsewhere, and silently welding endpoints here would hide a defect from the
/// code positioned to report it.
#[test]
fn contour_assembly_does_not_repair_gaps() {
    let disjoint = Contour::new(vec![
        segment(line()),
        segment(Curve2::Circle(Circle2 {
            frame: Frame2 {
                origin: Point2::new(50.0, 50.0),
                x: Vec2::X,
                y: Vec2::Y,
            },
            radius: 1.0,
        })),
    ]);
    assert_eq!(
        disjoint.len(),
        2,
        "both segments are kept exactly as supplied"
    );
    assert!(!disjoint.is_empty());
}

/// An empty contour is representable and reports itself as empty.
///
/// `Default` exists so a profile can be built incrementally; an empty contour
/// is not an error at this tier, it simply encloses nothing.
#[test]
fn an_empty_contour_is_representable() {
    let empty = Contour::default();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
}

/// `same_sense` is independent of the segment's curve and domain.
///
/// Orientation is carried by the flag, not inferred by reversing the interval,
/// so a consumer walking a contour must consult it rather than comparing
/// endpoints.
#[test]
fn segment_orientation_is_carried_by_the_flag() {
    let forward = segment(line());
    let reversed = ProfileSegment {
        same_sense: false,
        ..forward.clone()
    };
    assert_ne!(
        forward, reversed,
        "sense participates in equality: the two orientations are distinct values"
    );
    assert_eq!(
        forward.domain, reversed.domain,
        "reversing sense does not flip the stored interval"
    );
}

/// A contour profile separates its outer boundary from its holes.
///
/// Holes are a distinct field rather than extra contours in one list, so a
/// consumer cannot mistake a hole for an outer boundary through ordering
/// alone.
#[test]
fn holes_are_structurally_distinct_from_the_outer_boundary() {
    let profile = ContourProfile {
        outer: Contour::new(vec![segment(line())]),
        holes: vec![Contour::new(vec![segment(line())])],
    };
    assert_eq!(profile.outer.len(), 1);
    assert_eq!(profile.holes.len(), 1, "holes live in their own field");

    let solid = ContourProfile {
        outer: profile.outer.clone(),
        holes: Vec::new(),
    };
    assert_ne!(
        profile, solid,
        "a holed profile is not equal to its unholed counterpart"
    );
}
