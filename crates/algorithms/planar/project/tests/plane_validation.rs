use axiolid_core::{Point3, Tolerance, Vec3};
use axiolid_project::Plane;

/// A visibly skewed basis must not pass as an orthonormal projection plane.
///
/// The orthonormality test is a dot product of two unit vectors, which is
/// dimensionless. Comparing it against the LINEAR tolerance ties a pure
/// direction test to the model length unit.
#[test]
fn a_skewed_plane_is_refused_under_millimetre_tolerance() {
    // 0.5 milliradian of skew: x and y are no longer perpendicular.
    let skew = 5e-4;
    let plane = Plane::new(
        Point3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(skew, (1.0 - skew * skew).sqrt(), 0.0),
        Tolerance::MILLIMETRE,
    );
    // The skew is 5e-4: far above the 1e-9 angular tolerance, but below the
    // 1e-3 LINEAR tolerance millimetre-scale geometry carries. Deciding a
    // dimensionless property with the linear tolerance accepted this basis and
    // then silently produced wrong coordinates for every projected point.
    assert!(
        plane.is_err(),
        "a 0.5 mrad skewed basis must be refused regardless of the length unit"
    );
}
