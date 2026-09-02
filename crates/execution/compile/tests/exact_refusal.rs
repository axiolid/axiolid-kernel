//! The exact-compile path must refuse, not tessellate.
//!
//! `ReferenceExactCompiler` advertises `GRAPH_TO_EXACT_BREP` but implements no
//! construction family yet. The value under test is that asking for exactness
//! yields a typed refusal naming the capability -- never triangles. A mesh
//! returned here would be the silent-approximation failure this kernel exists
//! to avoid, so the refusal is the feature until a family lands.

use axiolid_contracts::{ExecutionOptions, GeomError, Operation};
use axiolid_core::{Tolerance, Vec2};
use axiolid_curve::{Curve2, Polyline2};
use axiolid_exact_compile_contract::ExactCompiler;
use axiolid_mesh_compile::ReferenceExactCompiler;
use axiolid_model::{GeometryGraphBuilder, OpenProfile};

#[test]
fn exact_request_refuses_rather_than_tessellating() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X, Vec2::ONE],
            closed: false,
        }))
        .unwrap();
    let profile = builder.push_value(OpenProfile::new(path)).unwrap();
    let graph = builder.finish(vec![profile]).unwrap();

    let error = ReferenceExactCompiler::new()
        .compile_exact(&graph, profile, &ExecutionOptions::new(Tolerance::METRE))
        .unwrap_err();

    assert!(
        matches!(
            error,
            GeomError::UnsupportedInput {
                operation: Operation::GraphCompilation,
                input: "open profile",
                ..
            }
        ),
        "exact compilation must refuse with a typed input-family diagnostic, got {error:?}"
    );
}
