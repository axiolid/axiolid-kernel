//! Line/line classification must distinguish topology, not return a maybe-point.

use axiolid_core::{Point2, Tolerance, Vec2};
use axiolid_linear::Line2;
use axiolid_linear_intersection::{
    line_line2, InputSide, LineLineIntersection2, LinearIntersectionError,
};

fn line(ox: f64, oy: f64, dx: f64, dy: f64) -> Line2 {
    Line2 {
        origin: Point2 { x: ox, y: oy },
        direction: Vec2 { x: dx, y: dy },
    }
}

#[test]
fn crossing_lines_report_the_point_and_both_parameters() {
    let result = line_line2(
        line(0.0, 0.0, 1.0, 0.0),
        line(2.0, -1.0, 0.0, 1.0),
        Tolerance::METRE,
    )
    .expect("axis-aligned lines cross");
    match result {
        LineLineIntersection2::Point {
            point,
            left_parameter,
            right_parameter,
        } => {
            assert_eq!(point, Point2 { x: 2.0, y: 0.0 });
            assert_eq!(left_parameter, 2.0);
            assert_eq!(right_parameter, 1.0);
        }
        other => panic!("expected a crossing, got {other:?}"),
    }
}

#[test]
fn parallel_and_coincident_are_different_answers() {
    let parallel = line_line2(
        line(0.0, 0.0, 1.0, 0.0),
        line(0.0, 1.0, 1.0, 0.0),
        Tolerance::METRE,
    )
    .expect("parallel lines classify");
    assert_eq!(parallel, LineLineIntersection2::Parallel);

    // Same line, different origin and direction magnitude: still one line.
    let coincident = line_line2(
        line(0.0, 0.0, 1.0, 0.0),
        line(5.0, 0.0, -3.0, 0.0),
        Tolerance::METRE,
    )
    .expect("coincident lines classify");
    assert_eq!(coincident, LineLineIntersection2::Coincident);
}

/// A near-parallel pair has a tiny float determinant. If the branch were a
/// naive `determinant.abs() < eps`, this would be misreported as parallel.
#[test]
fn near_parallel_lines_still_cross() {
    let result = line_line2(
        line(0.0, 0.0, 1.0, 0.0),
        line(0.0, 1.0, 1.0, 1e-13),
        Tolerance::METRE,
    )
    .expect("near-parallel lines still meet");
    assert!(
        matches!(result, LineLineIntersection2::Point { .. }),
        "a certified predicate must not collapse a crossing into Parallel: {result:?}"
    );
}

#[test]
fn a_zero_direction_is_refused_by_side() {
    assert_eq!(
        line_line2(
            line(0.0, 0.0, 0.0, 0.0),
            line(0.0, 0.0, 1.0, 0.0),
            Tolerance::METRE
        ),
        Err(LinearIntersectionError::DegenerateDirection {
            side: InputSide::Left
        })
    );
    assert_eq!(
        line_line2(
            line(0.0, 0.0, 1.0, 0.0),
            line(0.0, 0.0, 0.0, 0.0),
            Tolerance::METRE
        ),
        Err(LinearIntersectionError::DegenerateDirection {
            side: InputSide::Right
        })
    );
}

#[test]
fn non_finite_input_is_refused_rather_than_propagated() {
    assert_eq!(
        line_line2(
            line(f64::NAN, 0.0, 1.0, 0.0),
            line(0.0, 0.0, 0.0, 1.0),
            Tolerance::METRE
        ),
        Err(LinearIntersectionError::NonFiniteInput {
            side: InputSide::Left
        })
    );
    assert_eq!(
        line_line2(
            line(0.0, 0.0, 1.0, 0.0),
            line(0.0, 0.0, f64::INFINITY, 1.0),
            Tolerance::METRE
        ),
        Err(LinearIntersectionError::NonFiniteInput {
            side: InputSide::Right
        })
    );
}
