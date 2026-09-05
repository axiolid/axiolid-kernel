//! A Cartesian point trim must resolve end to end, not merely validate.
//!
//! Some formats can only state a trim as a pair of points: a three-point arc
//! knows its endpoints but not their parameters. Accepting such a curve into
//! the graph and refusing it at compile time leaves the geometry
//! representable but unusable.

use axiolid_contracts::ExecutionOptions;
use axiolid_core::{Frame3, Point3, Tolerance, Vec3};
use axiolid_curve::{Circle3, Curve3, Line3};
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{
    CurveRelation, GeometryGraphBuilder, GeometryNode, SolidOperation, TrimSelector,
    TrimmingPreference,
};

fn sweep_over(basis: GeometryNode, start: TrimSelector, end: TrimSelector) -> bool {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder.push(basis).expect("a basis curve is a valid node");
    let directrix = builder
        .push(GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![start],
            end: vec![end],
            sense_agreement: true,
            preference: TrimmingPreference::Cartesian,
        }))
        .expect("a point-trimmed curve is representable");
    let sweep = builder
        .push(GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            directrix,
            radius: 0.1,
            inner_radius: None,
            parameter_range: None,
            fillet_radius: None,
        }))
        .expect("a swept disk is a valid operation");
    let graph = builder.finish(vec![sweep]).expect("the graph validates");
    ReferenceMeshCompiler::new(BoolmeshBoolean::new())
        .compile_mesh(&graph, sweep, &ExecutionOptions::new(Tolerance::MILLIMETRE))
        .is_ok()
}

/// A line trimmed by two points on it compiles.
#[test]
fn a_point_trimmed_line_compiles() {
    assert!(
        sweep_over(
            GeometryNode::Curve3(Curve3::Line(Line3 {
                origin: Point3::ZERO,
                direction: Vec3::X,
            })),
            TrimSelector::Point3(Point3::new(1.0, 0.0, 0.0)),
            TrimSelector::Point3(Point3::new(4.0, 0.0, 0.0)),
        ),
        "a Cartesian point trim on a line must resolve to parameters"
    );
}

/// The three-point arc shape: a circle trimmed by its endpoints.
#[test]
fn a_point_trimmed_arc_compiles() {
    assert!(
        sweep_over(
            GeometryNode::Curve3(Curve3::Circle(Circle3 {
                frame: Frame3 {
                    origin: Point3::ZERO,
                    x: Vec3::X,
                    y: Vec3::Y,
                    z: Vec3::Z,
                },
                radius: 2.0,
            })),
            TrimSelector::Point3(Point3::new(2.0, 0.0, 0.0)),
            TrimSelector::Point3(Point3::new(0.0, 2.0, 0.0)),
        ),
        "a Cartesian point trim on a circle must resolve to parameters"
    );
}

/// A trim point that is not on the basis is refused, not snapped.
#[test]
fn a_point_off_the_basis_curve_is_refused() {
    assert!(
        !sweep_over(
            GeometryNode::Curve3(Curve3::Line(Line3 {
                origin: Point3::ZERO,
                direction: Vec3::X,
            })),
            TrimSelector::Point3(Point3::new(1.0, 0.0, 0.0)),
            TrimSelector::Point3(Point3::new(4.0, 9.0, 0.0)),
        ),
        "a trim point 9 units off the line must be refused, not projected"
    );
}
