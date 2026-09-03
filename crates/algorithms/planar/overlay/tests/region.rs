//! Region algebra: set ops, morphology, components (#47).
//!
//! Areas and component counts are derived from the geometry by hand, not
//! recorded from a previous run, so a regression cannot rewrite the
//! expectation into agreement with itself.

use axiolid_core::{Point2, Tolerance, Vec2};
use axiolid_overlay::{Polygon, Region, Ring};

fn tol() -> Tolerance {
    Tolerance::new(1e-9, 1e-9).expect("valid tolerance")
}

/// Axis-aligned rectangle as a single-polygon region.
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Region {
    Region::new(vec![rect_polygon(x0, y0, x1, y1)], tol()).expect("valid rectangle")
}

fn rect_polygon(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon {
        outer: Ring {
            points: vec![
                Point2::new(x0, y0),
                Point2::new(x1, y0),
                Point2::new(x1, y1),
                Point2::new(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

#[test]
fn area_and_components_are_reported() {
    // Two disjoint unit squares: area 2, two components.
    let left = rect(0.0, 0.0, 1.0, 1.0);
    let right = rect(3.0, 0.0, 4.0, 1.0);
    let both = left.union(&right, tol()).expect("valid union");

    assert!((both.area() - 2.0).abs() < 1e-9, "area was {}", both.area());
    assert_eq!(both.component_count(), 2);
    assert_eq!(both.evidence().polygons, 2);
    assert_eq!(both.boundary_rings().len(), 2);
}

#[test]
fn overlapping_union_merges_into_one_component() {
    // [0,2]x[0,1] and [1,3]x[0,1] overlap on [1,2]x[0,1]: union area 3.
    let a = rect(0.0, 0.0, 2.0, 1.0);
    let b = rect(1.0, 0.0, 3.0, 1.0);
    let merged = a.union(&b, tol()).expect("valid union");

    assert_eq!(merged.component_count(), 1);
    assert!(
        (merged.area() - 3.0).abs() < 1e-9,
        "area was {}",
        merged.area()
    );
}

#[test]
fn intersection_and_difference_have_the_expected_areas() {
    let a = rect(0.0, 0.0, 2.0, 2.0);
    let b = rect(1.0, 1.0, 3.0, 3.0);

    // Overlap is the unit square [1,2]x[1,2].
    let meet = a.intersection(&b, tol()).expect("valid intersection");
    assert!((meet.area() - 1.0).abs() < 1e-9, "area was {}", meet.area());

    // 4 total minus the 1 shared.
    let rest = a.difference(&b, tol()).expect("valid difference");
    assert!((rest.area() - 3.0).abs() < 1e-9, "area was {}", rest.area());
}

/// Free space split by a barrier has two components; without it, one.
///
/// This is the mutation evidence the issue asks for: the SAME query on the
/// same room returns a different component count purely because the barrier
/// is present or absent. A component counter stuck at 1 (or at the polygon
/// count of the input) passes neither half.
#[test]
fn removing_a_barrier_changes_the_component_count() {
    let room = rect(0.0, 0.0, 10.0, 4.0);
    // A full-height wall at x in [4,6] cuts the room in two.
    let barrier = rect(4.0, 0.0, 6.0, 4.0);

    let split = room.difference(&barrier, tol()).expect("valid difference");
    assert_eq!(
        split.component_count(),
        2,
        "a full-height barrier must separate the room"
    );
    // 40 total minus the 2x4 barrier.
    assert!(
        (split.area() - 32.0).abs() < 1e-9,
        "area was {}",
        split.area()
    );

    // The mutation: remove the barrier and the count must collapse to one.
    let unsplit = room.difference(&Region::empty(), tol()).expect("valid");
    assert_eq!(
        unsplit.component_count(),
        1,
        "without a barrier the room is a single component"
    );
    assert!((unsplit.area() - 40.0).abs() < 1e-9);
}

/// A barrier that does not reach the far wall leaves the space connected.
///
/// Guards the opposite error: a counter that keys off "an obstacle exists"
/// rather than actual connectivity would wrongly report two here.
#[test]
fn a_partial_barrier_does_not_separate_the_room() {
    let room = rect(0.0, 0.0, 10.0, 4.0);
    // Stops 1 unit short of the top edge, leaving a gap.
    let stub = rect(4.0, 0.0, 6.0, 3.0);

    let result = room.difference(&stub, tol()).expect("valid difference");
    assert_eq!(
        result.component_count(),
        1,
        "a gap at the top keeps the room connected"
    );
    assert!(
        (result.area() - 34.0).abs() < 1e-9,
        "area was {}",
        result.area()
    );
}

#[test]
fn erode_then_dilate_does_not_grow_the_region() {
    // The morphological opening is contained in the original: eroding then
    // dilating can only round convex corners off, never add material.
    let square = rect(0.0, 0.0, 10.0, 10.0);
    let original = square.area();

    let opened = square
        .erode(1.0, tol())
        .expect("valid erosion")
        .dilate(1.0, tol())
        .expect("valid dilation");

    assert!(
        opened.area() <= original + 1e-9,
        "opening grew the region: {} > {original}",
        opened.area()
    );
    // Corners are rounded by radius 1, so a little area is genuinely lost;
    // the bound keeps this honest rather than asserting equality.
    assert!(
        opened.area() > original - 1.0,
        "opening removed far more than corner rounding: {}",
        opened.area()
    );
}

#[test]
fn eroding_past_the_half_width_empties_the_region_and_says_so() {
    // A 2-wide strip cannot survive erosion by more than 1.
    let strip = rect(0.0, 0.0, 20.0, 2.0);
    let gone = strip.erode(1.5, tol()).expect("valid erosion");

    assert!(
        gone.is_empty(),
        "a 2-wide strip cannot survive erosion by 1.5"
    );
    assert!(
        gone.evidence().emptied,
        "the region must report that the operation destroyed it, not just return empty"
    );

    // An empty input eroded is empty but was NOT emptied by this operation.
    let nothing = Region::empty().erode(1.5, tol()).expect("valid erosion");
    assert!(nothing.is_empty());
    assert!(
        !nothing.evidence().emptied,
        "an already-empty region was not destroyed by this call"
    );
}

#[test]
fn dilation_grows_a_square_by_the_expected_area() {
    // A 10x10 square dilated by 1 becomes 12x12 minus the four corner
    // squares, plus four quarter-discs of radius 1: 144 - 4 + pi.
    let square = rect(0.0, 0.0, 10.0, 10.0);
    let grown = square.dilate(1.0, tol()).expect("valid dilation");

    let expected = 144.0 - 4.0 + core::f64::consts::PI;
    assert!(
        (grown.area() - expected).abs() < 0.05,
        "expected ~{expected}, got {}",
        grown.area()
    );
}

#[test]
fn translation_is_rigid_and_reversible() {
    let start = rect(0.0, 0.0, 3.0, 2.0);
    let moved = start.translate(Vec2::new(7.5, -2.25)).expect("valid");

    assert!((moved.area() - start.area()).abs() < 1e-12, "area changed");
    assert_eq!(moved.component_count(), start.component_count());

    let back = moved.translate(Vec2::new(-7.5, 2.25)).expect("valid");
    assert_eq!(back, start, "translate and back must be the identity");
}

#[test]
fn sweep_covers_the_swept_corridor() {
    // A 2x2 square swept 10 in x covers 2 wide by 12 long: area 24. Union of
    // the two end positions alone would be 8, so this fails loudly if the
    // corridor between them is not filled.
    let square = rect(0.0, 0.0, 2.0, 2.0);
    let swept = square
        .sweep(Vec2::new(10.0, 0.0), tol())
        .expect("valid sweep");

    assert_eq!(swept.component_count(), 1, "the sweep must be connected");
    assert!(
        (swept.area() - 24.0).abs() < 1e-9,
        "expected 24, got {}",
        swept.area()
    );
}

#[test]
fn a_hole_survives_set_algebra_and_is_reported() {
    // A 10x10 square with a 2x2 hole punched out: area 96, one hole.
    let plate = rect(0.0, 0.0, 10.0, 10.0);
    let punch = rect(4.0, 4.0, 6.0, 6.0);
    let holed = plate.difference(&punch, tol()).expect("valid difference");

    assert_eq!(holed.component_count(), 1);
    assert_eq!(holed.evidence().holes, 1, "the hole must be reported");
    assert!(
        (holed.area() - 96.0).abs() < 1e-9,
        "holes must subtract: got {}",
        holed.area()
    );
    // Outer ring plus one hole ring.
    assert_eq!(holed.boundary_rings().len(), 2);
}

#[test]
fn empty_operands_follow_set_identities() {
    let square = rect(0.0, 0.0, 2.0, 2.0);
    let nothing = Region::empty();

    assert_eq!(square.union(&nothing, tol()).expect("valid"), square);
    assert!(square
        .intersection(&nothing, tol())
        .expect("valid")
        .is_empty());
    assert_eq!(square.difference(&nothing, tol()).expect("valid"), square);
    assert!(nothing
        .difference(&square, tol())
        .expect("valid")
        .is_empty());
}

#[test]
fn construction_rejects_a_self_intersecting_ring() {
    let bowtie = Polygon {
        outer: Ring {
            points: vec![
                Point2::new(0.0, 0.0),
                Point2::new(2.0, 2.0),
                Point2::new(2.0, 0.0),
                Point2::new(0.0, 2.0),
            ],
        },
        holes: Vec::new(),
    };
    assert!(
        Region::new(vec![bowtie], tol()).is_err(),
        "a self-intersecting ring must be refused at construction"
    );
}

#[test]
fn a_negative_erosion_radius_is_refused() {
    let square = rect(0.0, 0.0, 2.0, 2.0);
    assert!(
        square.erode(-1.0, tol()).is_err(),
        "a negative erosion radius is a caller error, not a dilation"
    );
}
