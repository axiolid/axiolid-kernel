//! Segment classification must separate crossing, touching, and overlap.

use axiolid_core::{Point2, Tolerance};
use axiolid_linear::Segment2;
use axiolid_linear_intersection::{
    segment_segment2, InputSide, LinearIntersectionError, SegmentSegmentIntersection2,
};

fn segment(ax: f64, ay: f64, bx: f64, by: f64) -> Segment2 {
    Segment2 {
        start: Point2 { x: ax, y: ay },
        end: Point2 { x: bx, y: by },
    }
}

#[test]
fn transversal_crossing_reports_interior_parameters() {
    let result = segment_segment2(
        segment(0.0, 0.0, 2.0, 0.0),
        segment(1.0, -1.0, 1.0, 1.0),
        Tolerance::METRE,
    )
    .expect("crossing segments classify");
    match result {
        SegmentSegmentIntersection2::Point {
            point,
            left_parameter,
            right_parameter,
        } => {
            assert_eq!(point, Point2 { x: 1.0, y: 0.0 });
            assert_eq!(left_parameter, 0.5);
            assert_eq!(right_parameter, 0.5);
        }
        other => panic!("expected a crossing, got {other:?}"),
    }
}

/// Endpoint contact is a distinct topological fact from a transversal crossing
/// and must survive as an exact 0.0/1.0 parameter, not a rounded neighbour.
#[test]
fn endpoint_contact_is_reported_exactly() {
    let result = segment_segment2(
        segment(0.0, 0.0, 1.0, 0.0),
        segment(1.0, 0.0, 1.0, 1.0),
        Tolerance::METRE,
    )
    .expect("touching segments classify");
    match result {
        SegmentSegmentIntersection2::Point {
            point,
            left_parameter,
            right_parameter,
        } => {
            assert_eq!(point, Point2 { x: 1.0, y: 0.0 });
            assert_eq!(left_parameter, 1.0);
            assert_eq!(right_parameter, 0.0);
        }
        other => panic!("expected endpoint contact, got {other:?}"),
    }
}

#[test]
fn parallel_disjoint_and_collinear_disjoint_are_both_disjoint() {
    assert_eq!(
        segment_segment2(
            segment(0.0, 0.0, 1.0, 0.0),
            segment(0.0, 1.0, 1.0, 1.0),
            Tolerance::METRE
        )
        .expect("parallel segments classify"),
        SegmentSegmentIntersection2::Disjoint
    );
    assert_eq!(
        segment_segment2(
            segment(0.0, 0.0, 1.0, 0.0),
            segment(2.0, 0.0, 3.0, 0.0),
            Tolerance::METRE
        )
        .expect("collinear gap classifies"),
        SegmentSegmentIntersection2::Disjoint
    );
}

#[test]
fn collinear_overlap_reports_the_shared_span_on_both_operands() {
    let result = segment_segment2(
        segment(0.0, 0.0, 4.0, 0.0),
        segment(2.0, 0.0, 6.0, 0.0),
        Tolerance::METRE,
    )
    .expect("overlapping segments classify");
    match result {
        SegmentSegmentIntersection2::Overlap {
            left_interval,
            right_interval,
        } => {
            assert_eq!(left_interval.start, 0.5);
            assert_eq!(left_interval.end, 1.0);
            assert_eq!(right_interval.start, 0.0);
            assert_eq!(right_interval.end, 0.5);
        }
        other => panic!("expected an overlap, got {other:?}"),
    }
}

/// Collinear segments meeting at exactly one endpoint are a touch, not an
/// overlap: a zero-length shared span is not a span.
#[test]
fn collinear_endpoint_touch_is_a_point_not_an_overlap() {
    let result = segment_segment2(
        segment(0.0, 0.0, 1.0, 0.0),
        segment(1.0, 0.0, 2.0, 0.0),
        Tolerance::METRE,
    )
    .expect("collinear touch classifies");
    match result {
        SegmentSegmentIntersection2::Point { point, .. } => {
            assert_eq!(point, Point2 { x: 1.0, y: 0.0 });
        }
        other => panic!("expected a single touch point, got {other:?}"),
    }
}

#[test]
fn a_collapsed_segment_is_refused_by_side() {
    assert_eq!(
        segment_segment2(
            segment(1.0, 1.0, 1.0, 1.0),
            segment(0.0, 0.0, 1.0, 0.0),
            Tolerance::METRE
        ),
        Err(LinearIntersectionError::DegenerateDirection {
            side: InputSide::Left
        })
    );
}
