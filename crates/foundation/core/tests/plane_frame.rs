use axiolid_core::{PlaneFrame, PlaneFrameError, Point2, Point3, Tolerance, Vec3};

/// The property that makes the type worth having: a frame that exists maps
/// coordinates reversibly.
#[test]
fn project_and_lift_round_trip_for_points_on_the_plane() {
    let frame = PlaneFrame::new(
        Point3::new(3.0, -2.0, 7.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Tolerance::METRE,
    )
    .expect("an orthonormal basis builds a frame");

    for uv in [
        Point2::new(0.0, 0.0),
        Point2::new(1.5, -4.25),
        Point2::new(-1e6, 1e6),
    ] {
        let round_tripped = frame.project(frame.lift(uv));
        assert!(
            (round_tripped - uv).length() < 1e-9,
            "lift then project must return the original in-plane point"
        );
    }
}

/// Validity must not depend on the model length unit.
///
/// Orthonormality is a dimensionless property. This is the regression guard
/// for the defect that motivated the type: comparing a dot product against
/// the LINEAR tolerance made the same basis valid in millimetres and invalid
/// in metres.
#[test]
fn a_skewed_basis_is_refused_in_every_unit_system() {
    let skew = 5e-4;
    for tolerance in [Tolerance::METRE, Tolerance::MILLIMETRE] {
        let result = PlaneFrame::new(
            Point3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(skew, (1.0 - skew * skew).sqrt(), 0.0),
            tolerance,
        );
        assert_eq!(result, Err(PlaneFrameError::NotPerpendicular));
    }
}

/// Each rejection names its own cause, so a caller can act on it.
#[test]
fn each_invalid_basis_is_refused_by_name() {
    let t = Tolerance::METRE;
    let x = Vec3::new(1.0, 0.0, 0.0);
    let y = Vec3::new(0.0, 1.0, 0.0);

    assert_eq!(
        PlaneFrame::new(Point3::ZERO, x * 2.0, y, t),
        Err(PlaneFrameError::NotUnitLength)
    );
    assert_eq!(
        PlaneFrame::new(Point3::ZERO, x, x, t),
        Err(PlaneFrameError::Degenerate)
    );
    assert_eq!(
        PlaneFrame::new(Point3::new(f64::NAN, 0.0, 0.0), x, y, t),
        Err(PlaneFrameError::NonFiniteInput)
    );
}

/// `from_normal` must produce a frame that satisfies the same invariant it
/// would have been validated against, for every normal direction.
#[test]
fn from_normal_always_produces_a_valid_frame() {
    let normals = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-3.0, 0.5, 2.0),
        Vec3::new(1e-3, 0.0, 1.0),
    ];
    for normal in normals {
        let frame = PlaneFrame::from_normal(Point3::ZERO, normal, Tolerance::METRE)
            .expect("a finite non-zero normal defines a plane");
        // Re-validating the produced axes must succeed: from_normal and new
        // agree on what a valid frame is.
        PlaneFrame::new(
            frame.origin(),
            frame.x_axis(),
            frame.y_axis(),
            Tolerance::METRE,
        )
        .expect("from_normal must satisfy the constructor invariant");
        // The derived normal must be parallel to the requested one.
        let alignment = frame.normal().dot(normal.normalize());
        assert!(
            (alignment - 1.0).abs() < 1e-9,
            "frame normal must point along the requested normal, got {alignment}"
        );
    }
}

/// Off-plane points project onto the plane; the distance is reported
/// separately rather than silently lost.
#[test]
fn an_off_plane_point_projects_and_reports_its_distance() {
    let frame = PlaneFrame::ground();
    let point = Point3::new(2.0, 3.0, 5.0);
    assert_eq!(frame.project(point), Point2::new(2.0, 3.0));
    assert!((frame.signed_distance(point) - 5.0).abs() < 1e-12);
    assert!((frame.signed_distance(Point3::new(2.0, 3.0, -5.0)) + 5.0).abs() < 1e-12);
}
