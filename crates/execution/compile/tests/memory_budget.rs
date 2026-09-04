//! The compiler honours the caller's memory budget (v0.6 GAP).
//!
//! `scratch_requirement` has always honestly reported `Unbounded`, because
//! graph compilation caches every intermediate mesh and the peak is
//! data-dependent. By contract an unbounded requirement never fits a DECLARED
//! budget -- but nothing checked, so a caller could set a budget and watch the
//! compiler allocate straight past it.
//!
//! Budget enforcement existed in `dispatch/section.rs` and
//! `dispatch/boolean.rs` but not on the compile path, which is the path this
//! milestone is about.

use axiolid_contracts::{ExecutionOptions, GeomError};
use axiolid_core::{Tolerance, Vec3};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraphBuilder, GeometryNode, SolidOperation};
use axiolid_profile::{Profile, RectangleProfile};

fn compiler() -> ReferenceMeshCompiler<BoolmeshBoolean> {
    ReferenceMeshCompiler::new(BoolmeshBoolean::new())
}

fn rect(x: f64, y: f64) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    })
}

/// A simple box graph, the smallest thing that compiles to a mesh.
fn box_graph() -> (axiolid_model::GeometryGraph, axiolid_model::NodeId) {
    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(1.0, 1.0))).unwrap();
    let solid = b
        .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
            profile,
            direction: Vec3::Z,
            depth: 1.0,
        }))
        .unwrap();
    (b.finish(vec![solid]).unwrap(), solid)
}

/// Without a declared budget the compiler behaves exactly as before.
///
/// The budget is opt-in; adding the admission must not change the default.
#[test]
fn an_unbudgeted_compilation_still_succeeds() {
    let (graph, root) = box_graph();
    let options = ExecutionOptions::new(Tolerance::METRE);
    let mesh = compiler()
        .compile_mesh(&graph, root, &options)
        .expect("no budget declared, so nothing to exceed");
    assert!(!mesh.positions.is_empty());
}

/// A declared budget is REFUSED, because the requirement is unbounded.
///
/// Refusing a budget the compiler cannot prove it will respect is the honest
/// answer. Accepting it and hoping would make the budget advisory, and the
/// caller would have no way to tell the difference until it ran out of memory.
///
/// This is a generous budget deliberately: the point is not that the number is
/// too small, it is that an unbounded requirement fits NO declared budget.
#[test]
fn a_declared_budget_is_refused_because_the_requirement_is_unbounded() {
    let (graph, root) = box_graph();
    let options = ExecutionOptions::new(Tolerance::METRE).with_memory_budget(usize::MAX);

    let error = compiler()
        .compile_mesh(&graph, root, &options)
        .expect_err("an unbounded requirement cannot honour any declared budget");

    assert!(
        matches!(error, GeomError::BudgetExceeded { .. }),
        "expected a typed budget refusal, got {error:?}"
    );
}

/// The batch entry point is gated too.
///
/// Both call shapes share a cache, so gating only the single-root path would
/// leave the batch path as an unguarded way in.
#[test]
fn the_batch_entry_point_is_gated_as_well() {
    let (graph, root) = box_graph();
    let options = ExecutionOptions::new(Tolerance::METRE).with_memory_budget(usize::MAX);

    let mut destination = Vec::new();
    let error = compiler()
        .compile_mesh_batch_into(&graph, &[root], &options, &mut destination)
        .expect_err("the batch path must apply the same admission");

    assert!(matches!(error, GeomError::BudgetExceeded { .. }));
    assert!(
        destination.is_empty(),
        "a refused batch must not emit partial output"
    );
}
