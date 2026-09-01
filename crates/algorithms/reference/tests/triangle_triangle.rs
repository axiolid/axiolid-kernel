use axiolid_core::Point3;
use axiolid_reference::{triangle_triangle_relation, TriangleTriangleRelation};

fn point(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

#[test]
fn transverse_triangles_are_proper_in_either_order() {
    let horizontal = [
        point(-1.0, -1.0, 0.0),
        point(1.0, -1.0, 0.0),
        point(0.0, 1.0, 0.0),
    ];
    let vertical = [
        point(0.0, -0.5, -1.0),
        point(0.0, -0.5, 1.0),
        point(0.0, 0.75, 0.0),
    ];

    assert_eq!(
        triangle_triangle_relation(horizontal, vertical),
        TriangleTriangleRelation::Proper
    );
    assert_eq!(
        triangle_triangle_relation(vertical, horizontal),
        TriangleTriangleRelation::Proper
    );
}

#[test]
fn shared_vertex_is_touching() {
    let left = [
        point(0.0, 0.0, 0.0),
        point(2.0, 0.0, 0.0),
        point(0.0, 2.0, 0.0),
    ];
    let right = [
        point(0.0, 0.0, 0.0),
        point(-1.0, 0.0, 1.0),
        point(0.0, -1.0, 1.0),
    ];

    assert_eq!(
        triangle_triangle_relation(left, right),
        TriangleTriangleRelation::Touching
    );
}

#[test]
fn shared_hinge_edge_is_touching() {
    let horizontal = [
        point(-1.0, -1.0, 0.0),
        point(1.0, -1.0, 0.0),
        point(0.0, 1.0, 0.0),
    ];
    let vertical = [
        point(-1.0, -1.0, 0.0),
        point(1.0, -1.0, 0.0),
        point(0.0, -1.0, 1.0),
    ];

    assert_eq!(
        triangle_triangle_relation(horizontal, vertical),
        TriangleTriangleRelation::Touching
    );
}

#[test]
fn separated_triangles_are_disjoint() {
    let lower = [
        point(0.0, 0.0, 0.0),
        point(1.0, 0.0, 0.0),
        point(0.0, 1.0, 0.0),
    ];
    let upper = [
        point(0.0, 0.0, 2.0),
        point(1.0, 0.0, 2.0),
        point(0.0, 1.0, 2.0),
    ];

    assert_eq!(
        triangle_triangle_relation(lower, upper),
        TriangleTriangleRelation::Disjoint
    );
}

#[test]
fn coplanarity_is_explicit_without_claiming_overlap() {
    let first = [
        point(0.0, 0.0, 0.0),
        point(1.0, 0.0, 0.0),
        point(0.0, 1.0, 0.0),
    ];
    let second = [
        point(5.0, 5.0, 0.0),
        point(6.0, 5.0, 0.0),
        point(5.0, 6.0, 0.0),
    ];

    assert_eq!(
        triangle_triangle_relation(first, second),
        TriangleTriangleRelation::Coplanar
    );
}

#[test]
fn degenerate_input_is_explicit() {
    let degenerate = [
        point(0.0, 0.0, 0.0),
        point(1.0, 0.0, 0.0),
        point(2.0, 0.0, 0.0),
    ];
    let valid = [
        point(0.0, 0.0, 1.0),
        point(1.0, 0.0, 1.0),
        point(0.0, 1.0, 1.0),
    ];

    assert_eq!(
        triangle_triangle_relation(degenerate, valid),
        TriangleTriangleRelation::DegenerateTriangle
    );
}
