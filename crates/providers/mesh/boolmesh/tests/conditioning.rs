//! Conditioning is measured and reported, not silently absorbed (#81).
//!
//! Production booleans construct intersection coordinates in f64
//! (ADR 0045). On well-conditioned input that costs nothing measurable; as
//! operands approach coincidence, accuracy degrades and previously the
//! caller got a wrong answer with no signal at all.
//!
//! These tests pin the signal, not the arithmetic. Axiolid does not choose a
//! refusal threshold on the caller's behalf, so what must hold is that the
//! reported number tracks the actual conditioning of the operands.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::{BooleanOperator, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_boolean_contract::MeshBoolean;

/// Axis-aligned box as a closed triangle mesh.
fn box_at(min: [f64; 3], max: [f64; 3]) -> TriMesh {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let positions = vec![
        [x0, y0, z0].into(),
        [x1, y0, z0].into(),
        [x1, y1, z0].into(),
        [x0, y1, z0].into(),
        [x0, y0, z1].into(),
        [x1, y0, z1].into(),
        [x1, y1, z1].into(),
        [x0, y1, z1].into(),
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    TriMesh::new(positions, indices)
}

/// Two unit cubes overlapping by `d` along x: the issue's sliver probe.
fn overlapping_by(d: f64) -> (TriMesh, TriMesh) {
    let a = box_at([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_at([1.0 - d, 0.0, 0.0], [2.0 - d, 1.0, 1.0]);
    (a, b)
}

/// The reported ratio tracks the actual overlap across the sweep.
///
/// This is the sweep from #81. The reported conditioning must fall with the
/// overlap rather than staying constant — a flag that never moves carries no
/// information.
#[test]
fn the_reported_conditioning_tracks_the_sweep() {
    let provider = BoolmeshBoolean;
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);

    let mut previous: Option<f64> = None;
    for exponent in [3_i32, 6, 9, 12] {
        let d = 10_f64.powi(-exponent);
        let (a, b) = overlapping_by(d);
        let outcome = provider
            .boolean(&a, &b, BooleanOperator::Intersection, &options)
            .expect("intersection of two closed boxes");

        let reported = outcome
            .evidence
            .relative_overlap
            .expect("this provider measures conditioning");

        // The operands are unit cubes, so the diagonal scale is sqrt(3) for
        // each and the expected ratio is d / sqrt(3). Derived from the input,
        // not read back from the implementation.
        //
        // The tolerance widens with the sweep on purpose. Placing a box at
        // `1.0 - d` already rounds in f64: at d = 1e-12 the stored corner is
        // not exactly 1 - 1e-12, so the true overlap differs from the
        // requested one in the 4th significant digit. Demanding closer
        // agreement would be asserting that f64 can represent an operand it
        // cannot -- which is the very effect this issue is about. A relative
        // tolerance of 1e-3 still pins the value to well within an order of
        // magnitude, which is what a conditioning signal needs.
        let expected = d / 3.0_f64.sqrt();
        assert!(
            (reported - expected).abs() <= expected * 1e-3,
            "at overlap {d:e} expected ratio {expected:e}, reported {reported:e}"
        );

        if let Some(previous) = previous {
            assert!(
                reported < previous,
                "conditioning must fall as overlap shrinks: {reported:e} not below {previous:e}"
            );
        }
        previous = Some(reported);
    }
}

/// A well-conditioned operation reports a healthy ratio.
///
/// The counterpart to the sweep: the signal must distinguish good input from
/// bad, so it must NOT report a small number for cleanly overlapping boxes.
#[test]
fn a_well_conditioned_boolean_reports_a_healthy_ratio() {
    let provider = BoolmeshBoolean;
    let a = box_at([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_at([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);

    let outcome = provider
        .boolean(
            &a,
            &b,
            BooleanOperator::Intersection,
            &ExecutionOptions::new(Tolerance::MILLIMETRE),
        )
        .expect("intersection");

    let reported = outcome.evidence.relative_overlap.expect("measured");
    assert!(
        reported > 1e-6,
        "a half-overlap is well conditioned, got {reported:e}"
    );
}

/// Disjoint operands report no conditioning rather than a false alarm.
///
/// Nothing was intersected, so no intersection was ill conditioned. Reporting
/// a tiny ratio here would flag a perfectly clean no-op as dangerous, and
/// reporting a healthy one would invent a measurement that never happened.
#[test]
fn disjoint_operands_report_no_conditioning() {
    let provider = BoolmeshBoolean;
    let a = box_at([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let b = box_at([9.0, 9.0, 9.0], [10.0, 10.0, 10.0]);

    let outcome = provider
        .boolean(
            &a,
            &b,
            BooleanOperator::Union,
            &ExecutionOptions::new(Tolerance::MILLIMETRE),
        )
        .expect("union of disjoint boxes");

    assert!(
        outcome.evidence.relative_overlap.is_none(),
        "nothing intersected, so no conditioning was measured: {:?}",
        outcome.evidence.relative_overlap
    );
    assert_eq!(
        outcome.evidence.disjoint_tools, 1,
        "and the disjoint tool is still reported"
    );
}
