use axiolid_boolmesh::BoolmeshBoolean;
use axiolid_compile::ScalarCompiler;
use axiolid_core::{Tolerance, Vec2};
use axiolid_curve::{Curve2, Polyline2};
use axiolid_kernel::{ExecutionOptions, GeomError, GeometryCompiler, Operation};
use axiolid_model::{GeometryGraphBuilder, OpenProfile};

#[test]
fn authored_open_profile_requests_curve_evaluation_not_profile_triangulation() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X, Vec2::ONE],
            closed: false,
        }))
        .unwrap();
    let profile = builder.push_value(OpenProfile::new(path)).unwrap();
    let graph = builder.finish(vec![profile]).unwrap();

    let error = ScalarCompiler::new(BoolmeshBoolean::new())
        .compile(&graph, profile, &ExecutionOptions::new(Tolerance::METRE))
        .unwrap_err();

    assert!(matches!(
        error,
        GeomError::Unsupported {
            operation: Operation::CurveEvaluation,
            ..
        }
    ));
}
