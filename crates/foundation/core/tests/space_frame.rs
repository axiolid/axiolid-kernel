use axiolid_core::{FrameError, Point3, SpaceFrame, Tolerance, Vec3};

/// A tilted but valid frame, so the tests do not accidentally pass on the
/// identity basis.
fn tilted() -> SpaceFrame {
    let angle: f64 = 0.6;
    SpaceFrame::new(
        Point3::new(3.0, -1.0, 2.5),
        Vec3::new(angle.cos(), angle.sin(), 0.0),
        Vec3::new(-angle.sin(), angle.cos(), 0.0),
        Vec3::Z,
        Tolerance::METRE,
    )
    .expect("a rotation about z is a valid frame")
}

/// to_local and to_world must be exact inverses.
#[test]
fn local_and_world_round_trip() {
    let frame = tilted();
    for local in [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(2.5, -3.25, 1.75),
        Vec3::new(-1e5, 1e5, -1e5),
    ] {
        let back = frame.to_local(frame.to_world(local));
        assert!(
            (back - local).length() < 1e-9,
            "round trip drifted: {local:?} -> {back:?}"
        );
    }
}

/// A mirrored basis is orthonormal and must still be refused.
///
/// This is the property the three former call-site validators disagreed on:
/// two checked handedness, one did not. Unit length and perpendicularity are
/// both satisfied here, so nothing but an explicit triple-product test
/// separates this from a valid frame.
#[test]
fn a_left_handed_basis_is_refused() {
    let mirrored = SpaceFrame::new(Point3::ZERO, Vec3::X, Vec3::Y, -Vec3::Z, Tolerance::METRE);
    assert_eq!(mirrored, Err(FrameError::NotRightHanded));
}

/// Validity must not depend on the model length unit.
///
/// Orthonormality is dimensionless. Judging it with the linear tolerance made
/// the same skewed basis valid in millimetres and invalid in metres.
#[test]
fn a_skewed_basis_is_refused_in_every_unit_system() {
    let skew = 5e-4;
    for tolerance in [Tolerance::METRE, Tolerance::MILLIMETRE] {
        let frame = SpaceFrame::new(
            Point3::ZERO,
            Vec3::X,
            Vec3::new(skew, (1.0 - skew * skew).sqrt(), 0.0),
            Vec3::Z,
            tolerance,
        );
        assert_eq!(frame, Err(FrameError::NotPerpendicular));
    }
}

/// Each invalid basis names its own cause.
#[test]
fn each_invalid_basis_is_refused_by_name() {
    let t = Tolerance::METRE;
    assert_eq!(
        SpaceFrame::new(Point3::ZERO, Vec3::X * 2.0, Vec3::Y, Vec3::Z, t),
        Err(FrameError::NotUnitLength)
    );
    assert_eq!(
        SpaceFrame::new(
            Point3::new(f64::NAN, 0.0, 0.0),
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            t
        ),
        Err(FrameError::NonFiniteInput)
    );
}
