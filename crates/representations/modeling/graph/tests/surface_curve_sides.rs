//! A surface curve must record which p-curve lies on which surface.

use axiolid_core::{Vec2, Vec3};
use axiolid_curve::{Curve2, Curve3, Line2, Line3};
use axiolid_model::{
    CurveRelation, GeometryGraphBuilder, GeometryNode, MasterRepresentation, NodeId, SurfaceSides,
};
use axiolid_surface::{Plane, Surface};

/// Build a plane surface offset along z, so the two sides are distinct nodes.
fn plane_at(builder: &mut GeometryGraphBuilder, z: f64) -> NodeId {
    builder
        .push(GeometryNode::Surface(Surface::Plane(Plane {
            frame: axiolid_core::Frame3 {
                origin: Vec3::new(0.0, 0.0, z),
                x: Vec3::X,
                y: Vec3::Y,
                z: Vec3::Z,
            },
        })))
        .expect("a plane surface is a valid node")
}

/// A 2D parameter curve node.
fn pcurve(builder: &mut GeometryGraphBuilder) -> NodeId {
    builder
        .push(GeometryNode::Curve2(Curve2::Line(Line2 {
            origin: Vec2::ZERO,
            direction: Vec2::X,
        })))
        .expect("a 2d line is a valid node")
}

/// The master must be able to name the SECOND parametric side.
///
/// A surface curve is the intersection of two surfaces, and each side carries
/// its own parameter-space image. When the authoring
/// format says the second side governs, the graph has to be able to say so.
/// With an unordered `Vec` and a master that only knows "a parameter curve",
/// S1 and S2 are indistinguishable and a consumer must guess.
#[test]
fn the_master_can_name_the_second_parametric_side() {
    let mut builder = GeometryGraphBuilder::new();
    let curve_3d = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Vec3::ZERO,
            direction: Vec3::X,
        })))
        .expect("a 3d line is a valid node");
    let first = plane_at(&mut builder, 0.0);
    let second = plane_at(&mut builder, 1.0);
    let first_pcurve = pcurve(&mut builder);
    let second_pcurve = pcurve(&mut builder);

    let node = builder
        .push(GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
            curve_3d,
            sides: SurfaceSides::two(first, first_pcurve, second, second_pcurve),
            master: MasterRepresentation::ParameterCurveS2,
        }))
        .expect("a two-sided surface curve naming S2 is representable");

    let graph = builder.finish(vec![node]).expect("the graph is valid");
    let GeometryNode::CurveRelation(CurveRelation::SurfaceCurve { sides, master, .. }) =
        graph.get(node).expect("the node round-trips")
    else {
        panic!("the node must still be a surface curve");
    };
    assert_eq!(*master, MasterRepresentation::ParameterCurveS2);
    assert_eq!(sides.second(), Some((second, second_pcurve)));
}

/// Naming S2 as master without a second side is contradictory, not merely odd.
///
/// The old model accepted any node mix, so a file claiming the second p-curve
/// governs while supplying one p-curve validated cleanly and then silently
/// resolved to the wrong curve.
#[test]
fn naming_a_side_the_curve_does_not_have_is_refused() {
    let mut builder = GeometryGraphBuilder::new();
    let curve_3d = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Vec3::ZERO,
            direction: Vec3::X,
        })))
        .expect("a 3d line is a valid node");
    let surface = plane_at(&mut builder, 0.0);
    let only = pcurve(&mut builder);

    let error = builder
        .push(GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
            curve_3d,
            sides: SurfaceSides::one(surface, only),
            master: MasterRepresentation::ParameterCurveS2,
        }))
        .expect_err("S2 cannot govern a curve with only one parametric side");
    let text = format!("{error}");
    assert!(
        text.contains("second"),
        "the error must name the missing side, got: {text}"
    );
}

/// Not every edge has two parametric sides; one must stay expressible.
#[test]
fn a_single_sided_surface_curve_stays_expressible() {
    let mut builder = GeometryGraphBuilder::new();
    let curve_3d = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Vec3::ZERO,
            direction: Vec3::X,
        })))
        .expect("a 3d line is a valid node");
    let surface = plane_at(&mut builder, 0.0);
    let only = pcurve(&mut builder);

    builder
        .push(GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
            curve_3d,
            sides: SurfaceSides::one(surface, only),
            master: MasterRepresentation::ParameterCurveS1,
        }))
        .expect("a single-sided surface curve is legitimate");
}
