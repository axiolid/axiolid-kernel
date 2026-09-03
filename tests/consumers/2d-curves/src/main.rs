//! Minimal 2D plan-geometry consumer.
//!
//! Unit conversion stays an application concern: coordinates are converted to
//! the application's canonical unit before constructing Axiolid values.

use axiolid_core::{Frame2, Point2, Transform2, Vec2};
use axiolid_curve::{Circle2, Curve2};

const MILLIMETRES_PER_METRE: f64 = 1_000.0;

fn metres(millimetres: f64) -> f64 {
    millimetres / MILLIMETRES_PER_METRE
}

fn main() {
    let local = Point2::new(metres(1_000.0), metres(2_000.0));
    let placement = Transform2::from_translation(Vec2::new(3.0, -1.0));
    let world = placement.transform_point2(local);
    assert_eq!(world, Point2::new(4.0, 1.0));

    let curve = Curve2::Circle(Circle2 {
        frame: Frame2 {
            origin: world,
            x: Vec2::X,
            y: Vec2::Y,
        },
        radius: metres(500.0),
    });

    match curve {
        Curve2::Circle(circle) => {
            assert_eq!(circle.frame.origin, Point2::new(4.0, 1.0));
            assert_eq!(circle.radius, 0.5);
        }
        _ => unreachable!("fixture constructs a circle"),
    }

    println!("2d-curves ok");
}
