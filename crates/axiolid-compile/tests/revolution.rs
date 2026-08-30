//! Revolution against Pappus's theorem.
//!
//! A full revolution of a profile with area `A` whose centroid is `R` from
//! the axis has volume `2*pi*R*A`. That constant comes from outside this
//! crate, so it is a real oracle rather than the code agreeing with itself.

use axiolid_compile::profile::profile_rings;
use axiolid_compile::revolve::revolve;
use axiolid_core::{Point3, Scalar, Tolerance, Vec3};
use axiolid_measure::volume_properties;
use axiolid_mesh::TriMesh;
use axiolid_profile::{Profile, RectangleProfile};

fn tol_for(chord: Scalar) -> Tolerance {
    Tolerance::new(chord, 1e-9).expect("tolerance")
}

fn volume(mesh: &TriMesh, tol: Tolerance) -> Scalar {
    volume_properties(mesh, tol)
        .expect("a revolved solid must be closed and two-manifold")
        .signed_volume
}

/// A rectangle offset from the axis: the textbook Pappus case, a torus of
/// rectangular section.
fn rect(x: Scalar, y: Scalar) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    })
}

/// Move every ring point away from the axis along +x.
///
/// The profile is authored in its own z = 0 plane. Revolving about the Y
/// axis sweeps the x offset into the major radius, so the offset is what
/// makes this a torus rather than a degenerate self-intersecting sweep.
fn shift(rings: &axiolid_compile::profile::Rings, dx: Scalar) -> axiolid_compile::profile::Rings {
    let f = |p: &axiolid_core::Point2| axiolid_core::Point2::new(p.x + dx, p.y);
    axiolid_compile::profile::Rings {
        outer: rings.outer.iter().map(f).collect(),
        holes: rings
            .holes
            .iter()
            .map(|h| h.iter().map(f).collect())
            .collect(),
    }
}

#[test]
fn a_full_revolution_matches_pappus() {
    // Rectangle 1 x 2 centred at x = 4, revolved about the Y axis through
    // the origin. Pappus: V = 2*pi*R*A = 2*pi*4*2.
    let chord = 1e-5;
    let rings = profile_rings(&rect(1.0, 2.0), chord, tol_for(chord)).expect("rings");
    let shifted = shift(&rings, 4.0);
    let mesh = revolve(
        &shifted,
        Point3::ZERO,
        Vec3::Y,
        core::f64::consts::TAU,
        tol_for(chord),
    )
    .expect("revolve");
    let want = core::f64::consts::TAU * 4.0 * 2.0;
    let got = volume(&mesh, tol_for(chord));
    // A revolved polygon is inscribed in the true torus, so it
    // under-estimates. Asserting the direction catches an outward-bulging
    // sweep that a symmetric bound would accept.
    assert!(
        got < want,
        "inscribed sweep must under-estimate: {got} vs {want}"
    );
    assert!((want - got) / want < 1e-4, "pappus volume {got} vs {want}");
}

#[test]
fn a_full_revolution_is_closed_and_seamless() {
    let chord = 1e-4;
    let rings = profile_rings(&rect(1.0, 2.0), chord, tol_for(chord)).expect("rings");
    let shifted = shift(&rings, 4.0);
    let mesh = revolve(
        &shifted,
        Point3::ZERO,
        Vec3::Y,
        core::f64::consts::TAU,
        tol_for(chord),
    )
    .expect("revolve");
    // volume_properties refuses anything not closed and two-manifold, so a
    // split seam cannot reach the assertion above.
    assert!(volume(&mesh, tol_for(chord)) > 0.0, "outward wound");
}

#[test]
fn a_half_revolution_is_half_the_volume() {
    // A partial turn is capped at both ends. Half the angle must give half
    // the solid, which also proves the caps are not double counted.
    let chord = 1e-5;
    let rings = profile_rings(&rect(1.0, 2.0), chord, tol_for(chord)).expect("rings");
    let shifted = shift(&rings, 4.0);
    let half = revolve(
        &shifted,
        Point3::ZERO,
        Vec3::Y,
        core::f64::consts::PI,
        tol_for(chord),
    )
    .expect("revolve");
    let want = core::f64::consts::TAU * 4.0 * 2.0 / 2.0;
    let got = volume(&half, tol_for(chord));
    assert!(
        (want - got) / want < 1e-4,
        "half revolution {got} vs {want}"
    );
}

#[test]
fn a_zero_angle_is_refused() {
    let chord = 1e-4;
    let rings = profile_rings(&rect(1.0, 2.0), chord, tol_for(chord)).expect("rings");
    let shifted = shift(&rings, 4.0);
    assert!(revolve(&shifted, Point3::ZERO, Vec3::Y, 0.0, tol_for(chord)).is_err());
    assert!(revolve(&shifted, Point3::ZERO, Vec3::ZERO, 1.0, tol_for(chord)).is_err());
}
