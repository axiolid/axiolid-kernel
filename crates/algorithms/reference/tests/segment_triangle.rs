use axiolid_core::Point3;
use axiolid_reference::{segment_triangle_relation, SegmentTriangleRelation};

const TRIANGLE: [Point3; 3] = [
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(2.0, 0.0, 0.0),
    Point3::new(0.0, 2.0, 0.0),
];

#[test]
fn proper_crossing_is_certified() {
    assert_eq!(
        segment_triangle_relation(
            Point3::new(0.5, 0.5, -1.0),
            Point3::new(0.5, 0.5, 1.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Proper,
    );
}

#[test]
fn edge_and_vertex_contact_are_touching() {
    assert_eq!(
        segment_triangle_relation(
            Point3::new(0.0, 0.0, -1.0),
            Point3::new(0.0, 0.0, 1.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Touching,
    );
    assert_eq!(
        segment_triangle_relation(
            Point3::new(1.0, 1.0, -1.0),
            Point3::new(1.0, 1.0, 1.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Touching,
    );
}

#[test]
fn parallel_and_coplanar_segments_are_distinct() {
    assert_eq!(
        segment_triangle_relation(
            Point3::new(0.5, 0.5, 1.0),
            Point3::new(1.0, 0.5, 1.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Disjoint,
    );
    assert_eq!(
        segment_triangle_relation(
            Point3::new(0.25, 0.25, 0.0),
            Point3::new(1.0, 0.25, 0.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Coplanar,
    );
}

#[test]
fn plane_crossing_outside_the_triangle_is_disjoint() {
    assert_eq!(
        segment_triangle_relation(
            Point3::new(2.0, 2.0, -1.0),
            Point3::new(2.0, 2.0, 1.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Disjoint,
    );
    assert_eq!(
        segment_triangle_relation(
            Point3::new(2.0, 2.0, 1.0),
            Point3::new(2.0, 2.0, 0.0),
            TRIANGLE,
        ),
        SegmentTriangleRelation::Disjoint,
    );
}

#[test]
fn zero_length_or_collinear_inputs_are_explicit() {
    assert_eq!(
        segment_triangle_relation(Point3::ZERO, Point3::ZERO, TRIANGLE),
        SegmentTriangleRelation::DegenerateSegment,
    );
    assert_eq!(
        segment_triangle_relation(
            Point3::new(0.0, 0.0, -1.0),
            Point3::new(0.0, 0.0, 1.0),
            [Point3::ZERO, Point3::X, Point3::new(2.0, 0.0, 0.0)],
        ),
        SegmentTriangleRelation::DegenerateTriangle,
    );
}
