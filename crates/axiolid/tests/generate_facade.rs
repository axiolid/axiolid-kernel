//! The facade must expose solid generation as its own capability.
//!
//! `axiolid-compile` is not re-exported: the graph compiler is an
//! implementation detail. Generation is not -- a caller with an exact profile
//! and a path should be able to build a solid without adopting the DAG.

#![cfg(feature = "generate")]

use axiolid::core::{Tolerance, Vec3};
use axiolid::generate::extrude::extrude_profile;
use axiolid::generate::profile::profile_rings;
use axiolid::profile::{Profile, RectangleProfile};

#[test]
fn a_solid_can_be_generated_through_the_facade_alone() {
    let square = Profile::Rectangle(RectangleProfile {
        x: 2.0,
        y: 3.0,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    });
    let rings = profile_rings(&square, 1e-4, Tolerance::MILLIMETRE).expect("rings");
    let mesh = extrude_profile(&rings, Vec3::Z, 5.0, Tolerance::MILLIMETRE).expect("solid");

    // 2 x 3 x 5 box: 12 triangles, and a closed hull.
    assert!(!mesh.indices.is_empty(), "generation produced no geometry");
    assert_eq!(mesh.indices.len() % 3, 0, "indices must form triangles");
}

#[test]
fn the_generators_report_their_own_backend_identity() {
    // Not "scalar-compile": generation is no longer part of the compiler.
    assert_eq!(axiolid::generate::BACKEND_ID.as_str(), "scalar-generate");
}
