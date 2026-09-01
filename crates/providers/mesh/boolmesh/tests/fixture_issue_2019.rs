//! The ADR 0003 fixture case, as an executable regression.
//!
//! `test/fixtures/ifclite-geometry/issue_2019_wall_two_overlapping_openings.ifc`
//! is a 4x0.2x3 wall with three MUTUALLY OVERLAPPING openings, two rotated off
//! the wall's axes so the overlap regions are not grid-aligned. The dimensions
//! below are transcribed from that file.
//!
//! The mesh-level geometry is asserted here rather than through the IFC lowerer
//! so this gate stays meaningful before the parser path is wired up.

mod support;

use axiolid_core::Tolerance;
use axiolid_kernel::{ExecutionOptions, MeshBoolean};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use support::{boxx, volume};

/// Wall minus three entangled cutters must stay a valid solid and lose exactly
/// the material the cutters remove.
///
/// The expected volume comes from an independent Monte-Carlo integration run
/// during the ADR 0014 evaluation (4M samples: 2.0807 +/- MC noise), not from
/// this implementation, so the assertion is not self-confirming.
#[test]
fn wall_minus_three_overlapping_openings() {
    let wall = boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0);
    let openings = [
        boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0), // A, axis-aligned
        boxx(2.0, 0.1, 0.3, 1.0, 0.4, 1.2, 0.6435011), // B, ref dir (0.8, 0.6)
        boxx(1.8, 0.1, 0.6, 0.7, 0.5, 0.9, -0.9272952), // C, ref dir (0.6, -0.8)
    ];

    let result = BoolmeshBoolean::new()
        .subtract_many(&wall, &openings, &ExecutionOptions::new(Tolerance::METRE))
        .expect("wall minus three overlapping openings")
        .mesh;

    assert!(
        result.validate_structure().is_ok(),
        "result must be well formed"
    );

    let remaining = volume(&result);
    assert!(
        (remaining - 2.0807).abs() < 2e-3,
        "expected ~2.0807 from independent Monte-Carlo, got {remaining}"
    );
    assert!(remaining < volume(&wall), "openings must remove material");
}
