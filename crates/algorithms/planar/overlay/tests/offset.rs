//! Offset fixtures: round-trip, monotonicity, collapse and refusal (#42).
//!
//! Areas are checked against values computed by hand from the geometry rather
//! than recorded from a previous run of this code. A fixture that asserts what
//! the implementation currently produces cannot detect the implementation being
//! wrong.

use axiolid_core::{Point2, Tolerance};
use axiolid_overlay::{
    offset_polygons, polygon_area, stroke_polyline, total_area, CapStyle, JoinStyle, OverlayError,
    Polygon, Ring,
};

fn tolerance() -> Tolerance {
    Tolerance::new(1e-9, 1e-9).expect("valid tolerance")
}

/// Axis-aligned rectangle as a single polygon, counter-clockwise.
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
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

/// Square with a square hole: the case where outer and inner boundaries must
/// move in OPPOSITE directions under a single offset.
fn square_with_hole() -> Polygon {
    let mut polygon = rect(0.0, 0.0, 10.0, 10.0);
    polygon.holes.push(Ring {
        points: vec![
            Point2::new(4.0, 4.0),
            Point2::new(4.0, 6.0),
            Point2::new(6.0, 6.0),
            Point2::new(6.0, 4.0),
        ],
    });
    polygon
}

/// Bevel join, the only style with no free parameter to tune.
fn bevel() -> JoinStyle {
    JoinStyle::Bevel
}

#[test]
fn zero_distance_is_the_identity() {
    let input = vec![rect(0.0, 0.0, 4.0, 3.0)];
    let result = offset_polygons(&input, 0.0, bevel(), tolerance()).expect("valid offset");

    assert_eq!(result.polygons.len(), 1);
    // 4 x 3 = 12, exactly.
    assert!((total_area(&result.polygons) - 12.0).abs() < 1e-9);
    assert!(!result.evidence.collapsed);
}

#[test]
fn outset_grows_a_square_by_the_expected_area() {
    // A 10x10 square outset by 1 with bevelled corners is a 12x12 square with
    // the four corners cut off. Each cut removes a unit right triangle, so the
    // area is 144 - 4*(1/2) = 142. Derived from the geometry, not recorded.
    let input = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let result = offset_polygons(&input, 1.0, bevel(), tolerance()).expect("valid offset");

    assert_eq!(result.polygons.len(), 1);
    let area = total_area(&result.polygons);
    assert!(
        (area - 142.0).abs() < 1e-6,
        "expected 142 from the bevelled outset, got {area}"
    );
}

#[test]
fn inset_shrinks_a_square_by_the_expected_area() {
    // Inset by 1: an 8x8 square. Interior corners are not cut.
    let input = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let result = offset_polygons(&input, -1.0, bevel(), tolerance()).expect("valid offset");

    assert_eq!(result.polygons.len(), 1);
    let area = total_area(&result.polygons);
    assert!(
        (area - 64.0).abs() < 1e-6,
        "expected 64 from the inset, got {area}"
    );
}

#[test]
fn area_is_monotone_in_the_offset_distance() {
    let input = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let distances = [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
    let mut previous = f64::NEG_INFINITY;
    for distance in distances {
        let result = offset_polygons(&input, distance, bevel(), tolerance()).expect("valid offset");
        let area = total_area(&result.polygons);
        assert!(
            area > previous,
            "area must strictly increase with distance: {area} at {distance} did not exceed \
             {previous}"
        );
        previous = area;
    }
}

#[test]
fn an_inset_that_collapses_the_region_returns_empty_not_a_degenerate_ring() {
    // A 10x10 square has inradius 5, so an inset of 6 removes it entirely.
    let input = vec![rect(0.0, 0.0, 10.0, 10.0)];
    let result = offset_polygons(&input, -6.0, bevel(), tolerance()).expect("valid offset");

    assert!(
        result.polygons.is_empty(),
        "an over-inset region must vanish, got {} polygons",
        result.polygons.len()
    );
    assert!(
        result.evidence.collapsed,
        "collapse must be reported, otherwise the caller cannot distinguish it \
         from an empty input"
    );
}

#[test]
fn collapse_is_distinguishable_from_an_empty_input() {
    let empty = offset_polygons(&[], -6.0, bevel(), tolerance()).expect("empty input is valid");
    assert!(empty.polygons.is_empty());
    assert!(
        !empty.evidence.collapsed,
        "an empty input did not collapse; nothing was there to collapse"
    );
}

#[test]
fn outsetting_a_polygon_with_a_hole_shrinks_the_hole() {
    // The defining behaviour of offsetting a region rather than its rings: one
    // positive distance must grow the outer boundary and shrink the hole.
    let input = vec![square_with_hole()];
    let before = polygon_area(&input[0]);
    // 100 - 4 = 96.
    assert!((before - 96.0).abs() < 1e-9, "fixture area is {before}");

    let result = offset_polygons(&input, 0.5, bevel(), tolerance()).expect("valid offset");
    assert_eq!(result.polygons.len(), 1);
    let after = &result.polygons[0];

    assert_eq!(
        after.holes.len(),
        1,
        "the hole must survive an outset of half its inradius"
    );
    let hole_before = 4.0;
    let hole_after = axiolid_overlay::ring_area(&after.holes[0]);
    assert!(
        hole_after < hole_before,
        "outsetting the region must SHRINK the hole: {hole_after} was not below {hole_before}"
    );
}

#[test]
fn a_hole_can_be_closed_by_a_large_enough_outset() {
    // The 2x2 hole has inradius 1, so an outset of 1.5 closes it. The result
    // must have no hole at all rather than a collapsed degenerate ring.
    let input = vec![square_with_hole()];
    let result = offset_polygons(&input, 1.5, bevel(), tolerance()).expect("valid offset");

    assert_eq!(result.polygons.len(), 1);
    assert_eq!(
        result.polygons[0].holes.len(),
        0,
        "the hole must be gone, not present as a degenerate ring"
    );
    assert_eq!(result.evidence.output_holes, 0);
}

#[test]
fn a_non_finite_distance_is_refused() {
    let input = vec![rect(0.0, 0.0, 4.0, 3.0)];
    for distance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            offset_polygons(&input, distance, bevel(), tolerance()),
            Err(OverlayError::InvalidOffsetDistance),
            "distance {distance} must be refused"
        );
    }
}

#[test]
fn a_malformed_join_parameter_is_refused_separately_from_the_distance() {
    let input = vec![rect(0.0, 0.0, 4.0, 3.0)];
    assert_eq!(
        offset_polygons(
            &input,
            1.0,
            JoinStyle::Miter {
                angle_limit: f64::NAN
            },
            tolerance()
        ),
        Err(OverlayError::InvalidOffsetStyle)
    );
    assert_eq!(
        offset_polygons(
            &input,
            1.0,
            JoinStyle::Round {
                max_segment_ratio: -1.0
            },
            tolerance()
        ),
        Err(OverlayError::InvalidOffsetStyle)
    );
}

#[test]
fn a_self_intersecting_polygon_is_refused() {
    // A bow-tie. Offsetting it has no well-defined meaning, so the same ring
    // validation `overlay` applies must reject it here too.
    let bowtie = Polygon {
        outer: Ring {
            points: vec![
                Point2::new(0.0, 0.0),
                Point2::new(4.0, 4.0),
                Point2::new(4.0, 0.0),
                Point2::new(0.0, 4.0),
            ],
        },
        holes: Vec::new(),
    };
    assert_eq!(
        offset_polygons(&[bowtie], 1.0, bevel(), tolerance()),
        Err(OverlayError::SelfIntersection)
    );
}

#[test]
fn a_stroked_segment_has_the_expected_area() {
    // A straight segment of length 10 stroked at width 2 with butt caps is
    // exactly a 10 x 2 rectangle: area 20, computed from the geometry.
    let path = [Point2::new(0.0, 0.0), Point2::new(10.0, 0.0)];
    let result = stroke_polyline(&path, 2.0, bevel(), CapStyle::Butt, false).expect("valid stroke");

    assert_eq!(result.polygons.len(), 1);
    let area = total_area(&result.polygons);
    assert!(
        (area - 20.0).abs() < 1e-6,
        "expected 20 for a 10x2 stroke, got {area}"
    );
}

#[test]
fn square_caps_extend_the_stroke_by_half_the_width_at_each_end() {
    // Square caps add width/2 at each end: (10 + 2*1) x 2 = 24.
    let path = [Point2::new(0.0, 0.0), Point2::new(10.0, 0.0)];
    let butt = stroke_polyline(&path, 2.0, bevel(), CapStyle::Butt, false).expect("valid");
    let square = stroke_polyline(&path, 2.0, bevel(), CapStyle::Square, false).expect("valid");

    let butt_area = total_area(&butt.polygons);
    let square_area = total_area(&square.polygons);
    assert!(
        (square_area - 24.0).abs() < 1e-6,
        "expected 24 for square caps, got {square_area}"
    );
    assert!(
        square_area > butt_area,
        "square caps must cover more than butt caps"
    );
}

#[test]
fn width_is_the_full_stroke_width_not_a_half_width() {
    // Guards the convention stated in the module docs. A half-width reading
    // would give 10, and silently halving a clearance band is exactly the
    // failure this assertion exists to prevent.
    let path = [Point2::new(0.0, 0.0), Point2::new(10.0, 0.0)];
    let result = stroke_polyline(&path, 1.0, bevel(), CapStyle::Butt, false).expect("valid");
    let area = total_area(&result.polygons);
    assert!(
        (area - 10.0).abs() < 1e-6,
        "width 1 over length 10 must sweep area 10, got {area}"
    );
}

#[test]
fn a_self_intersecting_polyline_is_stroked_not_refused() {
    // A crossing path is a legitimate stroke input: the crossing is resolved by
    // the union of the swept region. Refusing it would decline a case the
    // operation genuinely handles.
    let path = [
        Point2::new(0.0, 0.0),
        Point2::new(10.0, 10.0),
        Point2::new(10.0, 0.0),
        Point2::new(0.0, 10.0),
    ];
    let result = stroke_polyline(&path, 1.0, bevel(), CapStyle::Butt, false)
        .expect("a crossing path is strokeable");

    assert!(
        !result.polygons.is_empty(),
        "the stroke must produce a region"
    );
    let area = total_area(&result.polygons);
    // Total path length is 10*sqrt(2) + 10 + 10*sqrt(2) ~= 38.28; at width 1
    // the swept area is at most that, and strictly less because the crossing
    // overlaps. A union that double-counted the overlap would exceed it.
    let upper = 10.0 * 2f64.sqrt() + 10.0 + 10.0 * 2f64.sqrt();
    assert!(
        area < upper,
        "overlap at the crossing must be unioned, not double-counted: {area} >= {upper}"
    );
    assert!(area > 0.0);
}

#[test]
fn a_closed_stroke_encloses_a_hole() {
    // Stroking a closed square path leaves the interior untouched, so the
    // result is an annulus: one polygon with one hole.
    let path = [
        Point2::new(0.0, 0.0),
        Point2::new(10.0, 0.0),
        Point2::new(10.0, 10.0),
        Point2::new(0.0, 10.0),
    ];
    let result = stroke_polyline(&path, 1.0, bevel(), CapStyle::Butt, true).expect("valid");

    assert_eq!(result.polygons.len(), 1);
    assert_eq!(
        result.polygons[0].holes.len(),
        1,
        "a closed stroke must leave the interior as a hole"
    );
}

#[test]
fn a_degenerate_or_malformed_stroke_is_refused() {
    let single = [Point2::new(0.0, 0.0)];
    assert_eq!(
        stroke_polyline(&single, 1.0, bevel(), CapStyle::Butt, false),
        Err(OverlayError::RingTooShort)
    );

    let path = [Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
    for width in [0.0, -1.0, f64::NAN] {
        assert_eq!(
            stroke_polyline(&path, width, bevel(), CapStyle::Butt, false),
            Err(OverlayError::InvalidOffsetDistance),
            "width {width} must be refused"
        );
    }

    let non_finite = [Point2::new(0.0, 0.0), Point2::new(f64::NAN, 0.0)];
    assert_eq!(
        stroke_polyline(&non_finite, 1.0, bevel(), CapStyle::Butt, false),
        Err(OverlayError::NonFinitePoint)
    );
}

#[test]
fn round_trip_outset_then_inset_recovers_the_original_area() {
    // Not an identity in general — offsetting rounds or bevels corners, so the
    // round trip loses the corner detail. For a convex polygon with bevel joins
    // the area returns to within the corner correction, which is what is
    // asserted rather than exact equality: claiming an exact round trip would
    // be claiming a property offsetting does not have.
    let input = vec![rect(0.0, 0.0, 20.0, 20.0)];
    let original = total_area(&input);

    let out = offset_polygons(&input, 2.0, bevel(), tolerance()).expect("outset");
    let back = offset_polygons(&out.polygons, -2.0, bevel(), tolerance()).expect("inset");
    let recovered = total_area(&back.polygons);

    // Corner correction for four bevelled corners at distance 2 is at most
    // 4 * (1/2) * 2 * 2 = 8.
    assert!(
        (recovered - original).abs() <= 8.0,
        "round trip lost more than the corner correction: {recovered} vs {original}"
    );
    assert!(
        recovered <= original + 1e-6,
        "a round trip must not grow the region: {recovered} > {original}"
    );
}
