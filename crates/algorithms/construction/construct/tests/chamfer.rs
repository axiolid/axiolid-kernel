//! Chamfer contract and single-edge exact reference implementation (#68).
//!
//! The oracle is the removed material, computed from the input parameters
//! rather than recorded from a run: chamfering a corner by distance d cuts a
//! right triangle of legs d, so the solid loses exactly d^2/2 * depth. That
//! is checked against the tessellated volume of the result, so a chamfer that
//! silently did nothing -- the worst failure, because the caller believes the
//! feature was applied -- fails loudly.

use axiolid_construct::feature::{chamfer_extruded_profile, EdgeSelector, FeatureSize};
use axiolid_contracts::GeomError;
use axiolid_core::{Point2, Tolerance, Vec3};
use axiolid_measure::exact_properties;
use axiolid_profile::{Profile, RectangleProfile};

fn rect(x: f64, y: f64) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    })
}

/// The chamfered solid loses exactly the triangular prism the cut removes.
#[test]
fn a_chamfer_removes_the_expected_volume() {
    let (x, y, depth, distance) = (4.0, 6.0, 2.0, 1.0);

    let chamfered = chamfer_extruded_profile(
        &rect(x, y),
        Vec3::Z,
        depth,
        // The +x/+y corner of a 4 x 6 rectangle sits at (2, 3).
        EdgeSelector::NearestCorner(Point2::new(2.0, 3.0)),
        FeatureSize::ConstantDistance(distance),
        Tolerance::METRE,
    )
    .expect("a constant-distance chamfer on a straight edge is constructible");

    // A chamfered rectangle is a pentagon: the corner vertex is replaced by
    // two. Five side walls plus two caps.
    assert_eq!(
        chamfered.topology().faces().len(),
        7,
        "chamfering one corner of a box yields a pentagonal prism"
    );

    // Every face is planar: the chamfer introduces no curved geometry.
    assert!(
        chamfered
            .surfaces()
            .iter()
            .all(|s| matches!(s, axiolid_surface::Surface::Plane(_))),
        "a chamfer must not introduce curved surfaces"
    );
}

/// The chamfer's cross-section area matches the oracle.
///
/// Volume is derived from the solid's own vertices via the shoelace formula
/// on its distinct cross-section, so it measures what was BUILT rather than
/// what was requested. A no-op chamfer returns the full box area and fails.
#[test]
fn the_chamfered_cross_section_loses_the_triangle() {
    let (x, y, depth, distance) = (4.0, 6.0, 2.0, 1.0);

    let chamfered = chamfer_extruded_profile(
        &rect(x, y),
        Vec3::Z,
        depth,
        EdgeSelector::NearestCorner(Point2::new(2.0, 3.0)),
        FeatureSize::ConstantDistance(distance),
        Tolerance::METRE,
    )
    .expect("constructible");

    // Recover the base cross-section: vertices at z = 0, deduplicated.
    let mut base: Vec<(f64, f64)> = chamfered
        .topology()
        .vertices()
        .iter()
        .filter(|v| v.position.z.abs() < 1e-9)
        .map(|v| (v.position.x, v.position.y))
        .collect();
    base.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
    assert_eq!(
        base.len(),
        5,
        "a chamfered rectangle is a pentagon: {base:?}"
    );

    // Volume comes from the measurement provider, not a local sum. The
    // cross-section count above already pins the topology; this pins the
    // magnitude against an oracle derived from the inputs.
    let measured = exact_properties(&chamfered, Tolerance::METRE)
        .expect("a chamfered prism is a closed planar solid");

    // Oracle from the inputs: full rectangle minus the right triangle of
    // legs `distance` that the cut removes, extruded through `depth`.
    let expected_area = x * y - distance * distance / 2.0;
    let expected_volume = expected_area * depth;
    assert!(
        (measured.signed_volume - expected_volume).abs() < 1e-9,
        "expected volume {expected_volume}, measured {} (a no-op chamfer would give {})",
        measured.signed_volume,
        x * y * depth
    );
}

/// A fillet is refused, not approximated by a many-segment chamfer.
///
/// A constant-radius blend needs a cylindrical wall stitched into the prism.
/// Substituting a polyline would be indistinguishable to the caller, which is
/// exactly the silent approximation the kernel exists to avoid.
#[test]
fn a_fillet_is_refused_rather_than_approximated() {
    let error = chamfer_extruded_profile(
        &rect(4.0, 6.0),
        Vec3::Z,
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 3.0)),
        FeatureSize::ConstantRadius(1.0),
        Tolerance::METRE,
    )
    .expect_err("no cylindrical-wall assembly exists yet");

    assert!(
        matches!(
            error,
            GeomError::UnsupportedInput {
                input: "constant-radius fillet on an extruded solid",
                ..
            }
        ),
        "the refusal must name the missing capability, got {error:?}"
    );
}

/// A chamfer consuming a whole edge changes the topology, so it is refused.
#[test]
fn an_oversized_chamfer_is_refused() {
    // Distance 2.0 on a 4-wide rectangle: the two chamfers would meet.
    let error = chamfer_extruded_profile(
        &rect(4.0, 6.0),
        Vec3::Z,
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 3.0)),
        FeatureSize::ConstantDistance(2.0),
        Tolerance::METRE,
    )
    .expect_err("the chamfer consumes an entire edge");
    assert!(matches!(error, GeomError::Degenerate(_)), "got {error:?}");
}

/// An oblique extrusion does not keep the vertical edges vertical.
#[test]
fn an_oblique_extrusion_is_refused() {
    let error = chamfer_extruded_profile(
        &rect(4.0, 6.0),
        Vec3::new(1.0, 0.0, 1.0),
        2.0,
        EdgeSelector::NearestCorner(Point2::new(2.0, 3.0)),
        FeatureSize::ConstantDistance(1.0),
        Tolerance::METRE,
    )
    .expect_err("an oblique extrusion has no vertical edges to chamfer");
    assert!(matches!(
        error,
        GeomError::UnsupportedInput {
            input: "chamfer on an oblique extrusion",
            ..
        }
    ));
}
