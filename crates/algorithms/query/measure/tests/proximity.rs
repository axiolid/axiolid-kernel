use axiolid_core::Point3;
use axiolid_measure::{
    closest_point_on_triangle, closest_points_on_segments, closest_points_on_triangles,
    ProximityError,
};

fn point(x: f64, y: f64, z: f64) -> Point3 {
    Point3::new(x, y, z)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn parallel_segments_return_deterministic_endpoint_witnesses() {
    let pair = closest_points_on_segments(
        [point(0.0, 0.0, 0.0), point(2.0, 0.0, 0.0)],
        [point(0.0, 3.0, 0.0), point(2.0, 3.0, 0.0)],
    )
    .expect("finite segments");

    assert_eq!(pair.point_a, point(0.0, 0.0, 0.0));
    assert_eq!(pair.point_b, point(0.0, 3.0, 0.0));
    assert_close(pair.distance_squared, 9.0);
}

#[test]
fn zero_length_segment_is_treated_as_a_point() {
    let pair = closest_points_on_segments(
        [point(1.0, 2.0, 3.0), point(1.0, 2.0, 3.0)],
        [point(0.0, 0.0, 3.0), point(4.0, 0.0, 3.0)],
    )
    .expect("finite segments");

    assert_eq!(pair.point_a, point(1.0, 2.0, 3.0));
    assert_eq!(pair.point_b, point(1.0, 0.0, 3.0));
    assert_close(pair.distance_squared, 4.0);
}

#[test]
fn triangle_projection_and_degenerate_input_are_explicit() {
    let triangle = [
        point(0.0, 0.0, 0.0),
        point(2.0, 0.0, 0.0),
        point(0.0, 2.0, 0.0),
    ];
    assert_eq!(
        closest_point_on_triangle(point(0.5, 0.5, 3.0), triangle),
        Ok(point(0.5, 0.5, 0.0))
    );
    assert_eq!(
        closest_point_on_triangle(
            Point3::ZERO,
            [Point3::ZERO, point(1.0, 0.0, 0.0), point(2.0, 0.0, 0.0)],
        ),
        Err(ProximityError::DegenerateTriangle)
    );
}

#[test]
fn intersecting_triangles_have_zero_metric_separation() {
    let horizontal = [
        point(0.0, 0.0, 0.0),
        point(2.0, 0.0, 0.0),
        point(0.0, 2.0, 0.0),
    ];
    let vertical = [
        point(0.5, 0.5, -1.0),
        point(0.5, 0.5, 1.0),
        point(1.5, 0.5, 0.0),
    ];

    let pair = closest_points_on_triangles(horizontal, vertical).expect("valid triangles");
    assert_close(pair.distance_squared, 0.0);
    assert_eq!(pair.point_a, pair.point_b);
}

#[test]
fn non_finite_input_fails_explicitly() {
    assert_eq!(
        closest_points_on_segments(
            [point(f64::NAN, 0.0, 0.0), Point3::ZERO],
            [Point3::ZERO, Point3::X],
        ),
        Err(ProximityError::NonFiniteInput)
    );
}

#[test]
fn separated_triangles_return_symmetric_witnesses() {
    let a = [
        point(0.0, 0.0, 0.0),
        point(2.0, 0.0, 0.0),
        point(0.0, 2.0, 0.0),
    ];
    let b = [
        point(5.0, 0.0, 0.0),
        point(7.0, 0.0, 0.0),
        point(5.0, 2.0, 0.0),
    ];

    let ab = closest_points_on_triangles(a, b).expect("valid triangles");
    let ba = closest_points_on_triangles(b, a).expect("valid triangles");

    assert_close(ab.distance_squared, 9.0);
    assert_close(ab.distance_squared, ba.distance_squared);
    assert_eq!(ab.point_a, ba.point_b);
    assert_eq!(ab.point_b, ba.point_a);
}
