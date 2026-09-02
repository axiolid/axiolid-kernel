//! The linear vocabulary must stay a stable, data-only surface.

use axiolid_core::{Point2, Point3, Vec2, Vec3};
use axiolid_linear::{Line2, Line3, Polyline2, Ray2, Ray3, Segment2, Segment3};

#[test]
fn lines_preserve_the_authored_direction() {
    // Normalising at construction would change the parameterisation a caller
    // reasons about, so a non-unit direction must survive unchanged.
    let line = Line2 {
        origin: Point2 { x: 1.0, y: 2.0 },
        direction: Vec2 { x: 3.0, y: 0.0 },
    };
    assert_eq!(line.direction.x, 3.0);
    assert_eq!(line.direction.y, 0.0);
}

#[test]
fn three_dimensional_line_is_available_without_the_curve_aggregate() {
    let line = Line3 {
        origin: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        direction: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    };
    assert_eq!(line.direction.z, 1.0);
}

#[test]
fn segments_carry_endpoint_identity() {
    let segment = Segment2 {
        start: Point2 { x: 0.0, y: 0.0 },
        end: Point2 { x: 1.0, y: 1.0 },
    };
    assert_eq!(segment.start.x, 0.0);
    assert_eq!(segment.end.y, 1.0);

    let spatial = Segment3 {
        start: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        end: Point3 {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        },
    };
    assert_eq!(spatial.end.z, 2.0);
}

#[test]
fn ray3_is_the_core_type_not_a_duplicate() {
    // A structurally identical but distinct `Ray3` would silently split the
    // vocabulary; this assignment only compiles while they are the same type.
    let core: axiolid_core::Ray3 = Ray3 {
        origin: Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        direction: Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    };
    assert_eq!(core.direction.x, 1.0);
}

#[test]
fn planar_ray_completes_the_pair() {
    let ray = Ray2 {
        origin: Point2 { x: 0.0, y: 0.0 },
        direction: Vec2 { x: 0.0, y: 1.0 },
    };
    assert_eq!(ray.direction.y, 1.0);
}

#[test]
fn polylines_preserve_order_and_closure() {
    let polyline = Polyline2 {
        points: vec![
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 1.0, y: 0.0 },
            Point2 { x: 1.0, y: 1.0 },
        ],
        closed: true,
    };
    assert_eq!(polyline.points.len(), 3);
    assert_eq!(polyline.points[1].x, 1.0);
    assert!(polyline.closed);
}
