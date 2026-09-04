use axiolid_contracts::{ExecutionOptions, GeomError, Operation};
use axiolid_core::{Tolerance, Vec3};
use axiolid_exact_compile_contract::ExactCompiler;
use axiolid_mesh_compile::ReferenceExactCompiler;
use axiolid_model::{GeometryGraphBuilder, GeometryNode, SolidOperation};
use axiolid_profile::{CircleProfile, Profile, RectangleProfile};
use axiolid_surface::Surface;
use axiolid_topology::audit_brep;

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::METRE)
}

fn extrusion_graph(
    profile: Profile,
    direction: Vec3,
    depth: f64,
) -> (axiolid_model::GeometryGraph, axiolid_model::NodeId) {
    let mut builder = GeometryGraphBuilder::new();
    let profile = builder
        .push(GeometryNode::Profile(profile))
        .expect("profile node");
    let extrusion = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
            profile,
            direction,
            depth,
        }))
        .expect("extrusion node");
    let graph = builder.finish(vec![extrusion]).expect("valid graph");
    (graph, extrusion)
}

#[test]
fn exact_rectangle_extrusion_compiles_to_closed_brep() {
    let (graph, root) = extrusion_graph(
        Profile::Rectangle(RectangleProfile {
            x: 4.0,
            y: 2.0,
            thickness: Some(0.25),
            outer_radius: None,
            inner_radius: None,
        }),
        Vec3::new(0.25, 0.5, 1.0),
        3.0,
    );

    let exact = ReferenceExactCompiler::new()
        .compile_exact(&graph, root, &options())
        .expect("exact rectangle result");

    assert!(audit_brep(exact.topology()).is_closed_manifold());
    assert_eq!(exact.topology().faces().len(), 10);
    assert!(exact
        .surfaces()
        .iter()
        .all(|surface| matches!(surface, Surface::Plane(_))));
}

#[test]
fn exact_circle_extrusion_leaves_compiler_as_a_cylinder() {
    let (graph, root) = extrusion_graph(
        Profile::Circle(CircleProfile {
            radius: 2.0,
            thickness: None,
        }),
        Vec3::Z,
        5.0,
    );

    let exact = ReferenceExactCompiler::new()
        .compile_exact(&graph, root, &options())
        .expect("exact cylinder result");

    assert!(audit_brep(exact.topology()).is_closed_manifold());
    assert_eq!(
        exact
            .surfaces()
            .iter()
            .filter(|surface| matches!(surface, Surface::Cylinder(_)))
            .count(),
        1
    );
}

#[test]
fn exact_batch_preserves_order_and_duplicate_results() {
    let (graph, root) = extrusion_graph(
        Profile::Rectangle(RectangleProfile {
            x: 2.0,
            y: 1.0,
            thickness: None,
            outer_radius: None,
            inner_radius: None,
        }),
        Vec3::Z,
        1.0,
    );

    let exact = ReferenceExactCompiler::new()
        .compile_exact_batch(&graph, &[root, root], &options())
        .expect("shared exact batch");

    assert_eq!(exact.len(), 2);
    assert_eq!(exact[0], exact[1]);
}

#[test]
fn constructor_refusal_is_owned_by_graph_compilation() {
    let (graph, root) = extrusion_graph(
        Profile::Circle(CircleProfile {
            radius: 1.0,
            thickness: Some(0.2),
        }),
        Vec3::Z,
        1.0,
    );

    let error = ReferenceExactCompiler::new()
        .compile_exact(&graph, root, &options())
        .expect_err("annular exact extrusion is not implemented");

    assert!(matches!(
        error,
        GeomError::UnsupportedInput {
            backend,
            operation: Operation::GraphCompilation,
            input: "annular circle extrusion",
        } if backend == ReferenceExactCompiler::ID
    ));
}

#[test]
fn unsupported_exact_operation_names_its_family() {
    let mut builder = GeometryGraphBuilder::new();
    let profile = builder
        .push(GeometryNode::Profile(Profile::Rectangle(
            RectangleProfile {
                x: 2.0,
                y: 1.0,
                thickness: None,
                outer_radius: None,
                inner_radius: None,
            },
        )))
        .unwrap();
    let revolution = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Revolution {
            profile,
            axis_origin: Vec3::ZERO,
            axis_direction: Vec3::Z,
            angle: std::f64::consts::TAU,
        }))
        .unwrap();
    let graph = builder.finish(vec![revolution]).unwrap();

    // Exact revolution now exists, but this fixture revolves about the
    // profile's own local z with the axis running through it: the profile
    // crosses the axis, so the swept solid is not an annular tube. The
    // refusal must therefore name that specific geometry rather than claim
    // revolution is unimplemented.
    let error = ReferenceExactCompiler::new()
        .compile_exact(&graph, revolution, &options())
        .expect_err("this profile crosses its revolution axis");

    let GeomError::UnsupportedInput { input, .. } = error else {
        panic!("expected a typed input-family refusal, got {error:?}");
    };
    assert!(
        input.contains("axis"),
        "the refusal must name the axis geometry that blocks it, got {input:?}"
    );
}

#[test]
fn an_offset_revolution_compiles_to_an_exact_annular_tube() {
    // Same graph shape an IFC revolved-area solid produces, with the axis
    // clear of the profile so the result is an annulus.
    let mut builder = GeometryGraphBuilder::new();
    let profile = builder
        .push(GeometryNode::Profile(Profile::Rectangle(
            RectangleProfile {
                x: 2.0,
                y: 3.0,
                thickness: None,
                outer_radius: None,
                inner_radius: None,
            },
        )))
        .unwrap();
    let revolution = builder
        .push(GeometryNode::SolidOperation(SolidOperation::Revolution {
            profile,
            axis_origin: axiolid_core::Point3::new(5.0, 0.0, 0.0),
            axis_direction: Vec3::Y,
            angle: std::f64::consts::TAU,
        }))
        .unwrap();
    let graph = builder.finish(vec![revolution]).unwrap();

    let exact = ReferenceExactCompiler::new()
        .compile_exact(&graph, revolution, &options())
        .expect("an offset full-turn revolution is exactly constructible");

    // Analytic surfaces, not tessellation: two cylinders and two planes.
    assert_eq!(exact.topology().faces().len(), 4);
    let cylinders = exact
        .surfaces()
        .iter()
        .filter(|s| matches!(s, axiolid_surface::Surface::Cylinder(_)))
        .count();
    assert_eq!(cylinders, 2, "an annular tube has two cylindrical walls");
}
