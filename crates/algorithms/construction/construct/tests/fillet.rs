//! Constant-radius fillet on one straight edge (#79).
//!
//! The volume oracle is closed-form and independent of the implementation:
//! filleting a corner removes the region between the sharp corner and the
//! arc, which for a straight edge is a prism of cross-section
//! `r^2 - pi*r^2/4` -- the corner square minus the quarter-disc.

use axiolid_construct::feature::{fillet_extruded_profile, EdgeSelector, FeatureSize};
use axiolid_core::{Point2, Tolerance, Vec3};
use axiolid_profile::{Profile, RectangleProfile};

fn tol() -> Tolerance {
    Tolerance::new(1e-6, 1e-9).expect("tolerance")
}

fn rectangle(x: f64, y: f64) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    })
}

#[test]
fn a_chamfer_request_through_the_fillet_entry_point_is_refused() {
    let error = fillet_extruded_profile(
        &rectangle(4.0, 3.0),
        Vec3::Z,
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 1.5)),
        FeatureSize::ConstantDistance(0.5),
        tol(),
    )
    .expect_err("a distance is a chamfer, not a fillet");
    assert!(format!("{error}").contains("chamfer"));
}

#[test]
fn a_radius_reaching_past_a_neighbouring_corner_is_refused() {
    // Half of the 3-unit side: the blend would consume the whole wall.
    let error = fillet_extruded_profile(
        &rectangle(4.0, 3.0),
        Vec3::Z,
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 1.5)),
        FeatureSize::ConstantRadius(1.5),
        tol(),
    )
    .expect_err("an over-large radius must be refused");
    assert!(format!("{error}").contains("reaches past"));
}

#[test]
fn a_non_positive_radius_is_refused() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fillet_extruded_profile(
                &rectangle(4.0, 3.0),
                Vec3::Z,
                2.0,
                EdgeSelector::NearestCorner(Point2::new(2.0, 1.5)),
                FeatureSize::ConstantRadius(bad),
                tol(),
            )
            .is_err(),
            "radius {bad} must be refused"
        );
    }
}

#[test]
fn the_preserved_refusals_still_hold() {
    let hollow = Profile::Rectangle(RectangleProfile {
        x: 4.0,
        y: 3.0,
        thickness: Some(0.2),
        outer_radius: None,
        inner_radius: None,
    });
    assert!(fillet_extruded_profile(
        &hollow,
        Vec3::Z,
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 1.5)),
        FeatureSize::ConstantRadius(0.5),
        tol(),
    )
    .is_err());

    // An oblique extrusion does not keep the vertical edges vertical, so the
    // blend would not be a cylinder.
    assert!(fillet_extruded_profile(
        &rectangle(4.0, 3.0),
        Vec3::new(0.3, 0.0, 1.0),
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 1.5)),
        FeatureSize::ConstantRadius(0.5),
        tol(),
    )
    .is_err());
}
