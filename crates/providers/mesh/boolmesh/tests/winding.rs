//! Gate 1: orientation. An inside-out solid must be refused, not computed.
//!
//! Rationale in `src/convert.rs`: an inverted mesh passes every structural and
//! manifold check, and turns `Difference` into `Union` silently. The ADR 0014
//! evaluation hit this for real.

mod support;

use axiolid_core::BooleanOperator;
use axiolid_core::Tolerance;
use axiolid_kernel::{ExecutionOptions, GeomError, MeshBoolean};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use support::{boxx, inverted, volume};

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

fn wall() -> axiolid_mesh::TriMesh {
    boxx(2.0, 0.1, 0.0, 4.0, 0.2, 3.0, 0.0)
}

/// The fixture builder itself must be outward-oriented, or every other test in
/// this crate is testing the wrong thing.
#[test]
fn the_fixture_box_is_outward_oriented() {
    let mesh = wall();
    assert!((volume(&mesh) - 2.4).abs() < 1e-12, "wall volume");
    // Accepted by the provider, which only accepts outward orientation.
    let tool = boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0);
    assert!(BoolmeshBoolean::new()
        .boolean(&mesh, &tool, BooleanOperator::Difference, &options())
        .is_ok());
}

/// An inverted subject is structurally valid and manifold. It must still be
/// refused, naming which argument was wrong.
#[test]
fn an_inside_out_subject_is_refused_before_any_computation() {
    let subject = inverted(&wall());
    let tool = boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0);

    // The precondition that makes this test meaningful: the mesh is *not*
    // malformed in any way a structural check would catch.
    assert!(subject.validate_structure().is_ok());

    let error = BoolmeshBoolean::new()
        .boolean(&subject, &tool, BooleanOperator::Difference, &options())
        .expect_err("an inside-out subject must be refused");

    match error {
        GeomError::InvalidInput(detail) => {
            assert!(
                detail.starts_with("subject:"),
                "must name the argument: {detail}"
            );
            assert!(
                detail.contains("inside-out"),
                "must name the cause: {detail}"
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

/// Same for the tool, and the diagnostic must distinguish it from the subject.
#[test]
fn an_inside_out_tool_is_refused_and_named_distinctly() {
    let tool = inverted(&boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0));
    let error = BoolmeshBoolean::new()
        .boolean(&wall(), &tool, BooleanOperator::Difference, &options())
        .expect_err("an inside-out tool must be refused");

    match error {
        GeomError::InvalidInput(detail) => assert!(
            detail.starts_with("tool:"),
            "must name the tool, not the subject: {detail}"
        ),
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

/// The bug this gate exists to prevent, stated as an outcome rather than a
/// mechanism: subtraction must never increase enclosed volume.
#[test]
fn subtraction_never_increases_volume() {
    let subject = wall();
    let before = volume(&subject);
    let result = BoolmeshBoolean::new()
        .boolean(
            &subject,
            &boxx(1.5, 0.1, 0.3, 1.0, 0.4, 1.2, 0.0),
            BooleanOperator::Difference,
            &options(),
        )
        .expect("difference")
        .mesh;
    let after = volume(&result);
    assert!(
        after < before,
        "difference must remove material: {before} -> {after}"
    );
}
