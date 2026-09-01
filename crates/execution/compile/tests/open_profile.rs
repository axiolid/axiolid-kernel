use axiolid_contracts::{ExecutionOptions, GeomError, Operation};
use axiolid_core::{Tolerance, Vec2};
use axiolid_curve::{Curve2, Polyline2};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshCompiler;
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

    let error = ReferenceMeshCompiler::new(BoolmeshBoolean::new())
        .compile_mesh(&graph, profile, &ExecutionOptions::new(Tolerance::METRE))
        .unwrap_err();

    assert!(matches!(
        error,
        GeomError::Unsupported {
            operation: Operation::CurveEvaluation,
            ..
        }
    ));
}
