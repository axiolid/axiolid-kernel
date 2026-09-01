use axiolid_compile::ScalarCompiler;
use axiolid_core::{Frame3, Point3, Tolerance, Vec3};
use axiolid_kernel::{ExecutionOptions, GeometryCompiler, MeshPlaneSectionRegistry, SectionLimits};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_model::{GeometryGraphBuilder, GeometryNode, SolidOperation};
use axiolid_profile::{Profile, RectangleProfile};
use axiolid_reference::ScalarSection;

#[test]
fn body_graph_compiles_and_sections_into_plan_linework() {
    let mut builder = GeometryGraphBuilder::new();
    let profile = builder
        .push(GeometryNode::Profile(Profile::Rectangle(
            RectangleProfile {
                x: 4.0,
                y: 0.2,
                thickness: None,
                outer_radius: None,
                inner_radius: None,
            },
        )))
        .expect("profile");
    let body = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
            profile,
            direction: Vec3::Z,
            depth: 3.0,
        }))
        .expect("body");
    let graph = builder.finish(vec![body]).expect("graph");
    let options = ExecutionOptions::new(Tolerance::METRE);
    let mesh = ScalarCompiler::new(BoolmeshBoolean::new())
        .compile(&graph, body, &options)
        .expect("compile Body DAG");

    let mut sections = MeshPlaneSectionRegistry::new();
    sections.register(0, ScalarSection);
    let frame = Frame3 {
        origin: Point3::new(0.0, 0.0, 1.2),
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    };
    let result = sections
        .section(
            &mesh,
            frame,
            SectionLimits::new(100_000, 100_000, 100_000, 1_000),
            &options,
        )
        .expect("section Body mesh");
    assert_eq!(result.contours.len(), 1);
    assert_eq!(result.contours[0].points.len(), 4);
    assert!(result.contours[0].is_closed());
    assert!(result.evidence.is_derived_from_input_mesh());
    assert_eq!(result.evidence.source_triangles, 12);
    assert_eq!(result.evidence.output_vertices, 4);
}
