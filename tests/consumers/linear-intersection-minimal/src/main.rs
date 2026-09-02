//! Minimal line-query application.
//!
//! Its purpose is to be COMPILED, not to be useful: `cargo xtask architecture
//! closure check` resolves this manifest to prove the internal package closure
//! stays at core + linear + linear-intersection + predicates.

use axiolid_linear::{Line2, Point2, Segment2, Vec2};
use axiolid_linear_intersection::{line_line2, segment_segment2, Tolerance};

fn main() {
    let crossing = line_line2(
        Line2 {
            origin: Point2 { x: 0.0, y: 0.0 },
            direction: Vec2 { x: 1.0, y: 0.0 },
        },
        Line2 {
            origin: Point2 { x: 1.0, y: -1.0 },
            direction: Vec2 { x: 0.0, y: 1.0 },
        },
        Tolerance::METRE,
    );
    println!("line/line: {crossing:?}");

    let overlap = segment_segment2(
        Segment2 {
            start: Point2 { x: 0.0, y: 0.0 },
            end: Point2 { x: 4.0, y: 0.0 },
        },
        Segment2 {
            start: Point2 { x: 2.0, y: 0.0 },
            end: Point2 { x: 6.0, y: 0.0 },
        },
        Tolerance::METRE,
    );
    println!("segment/segment: {overlap:?}");
}
