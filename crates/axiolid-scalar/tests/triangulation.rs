//! Gate: a triangulation must cover exactly the input area, no more, no less.
//!
//! Area conservation is the triangulation-invariant check. Counting triangles
//! only proves a fan was emitted; summing their areas proves it covers the
//! polygon, and for holes it proves the holes were actually subtracted.

use axiolid_core::Point2;
use axiolid_scalar::{signed_area2, triangulate_simple};

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

fn tri_area(a: Point2, b: Point2, c: Point2) -> f64 {
    ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)) / 2.0
}

fn covered_area(verts: &[Point2], tris: &[[u32; 3]]) -> f64 {
    tris.iter()
        .map(|t| {
            tri_area(
                verts[t[0] as usize],
                verts[t[1] as usize],
                verts[t[2] as usize],
            )
        })
        .sum()
}

#[test]
fn a_square_triangulates_into_two_triangles_covering_its_area() {
    let square = vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)];
    let tris = triangulate_simple(&square).expect("triangulate");
    assert_eq!(tris.len(), 2, "n-gon yields n-2 triangles");
    assert!((covered_area(&square, &tris) - 16.0).abs() < 1e-12);
}

/// An L-shape has a reflex vertex, which naive fan triangulation gets wrong.
#[test]
fn a_reflex_vertex_is_handled_without_covering_outside_area() {
    let l = vec![
        p(0.0, 0.0),
        p(4.0, 0.0),
        p(4.0, 2.0),
        p(2.0, 2.0),
        p(2.0, 4.0),
        p(0.0, 4.0),
    ];
    let tris = triangulate_simple(&l).expect("triangulate");
    assert_eq!(tris.len(), 4);
    let expected = signed_area2(&l) / 2.0;
    assert!((covered_area(&l, &tris) - expected).abs() < 1e-12);
    assert!((expected - 12.0).abs() < 1e-12, "L area is 12");
}

#[test]
fn a_degenerate_ring_is_refused_not_silently_empty() {
    assert!(triangulate_simple(&[p(0.0, 0.0), p(1.0, 1.0)]).is_err());
    let collinear = vec![p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)];
    assert!(triangulate_simple(&collinear).is_err());
}
