use axiolid_core::{Vec2, Vec3};
use axiolid_curve::{Curve2, Curve3, Line2, Line3, Polyline2};
use axiolid_model::{
    CurveRelation, GeometryGraphBuilder, GeometryNode, GraphError, OpenProfile, SolidOperation,
    TrimSelector, TrimmingPreference,
};

fn line2() -> Curve2 {
    Curve2::Line(Line2 {
        origin: Vec2::ZERO,
        direction: Vec2::X,
    })
}

#[test]
fn authored_open_profile_preserves_an_exact_relational_curve() {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder.push(GeometryNode::Curve2(line2())).unwrap();
    let trimmed = builder
        .push(GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(1.0)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }))
        .unwrap();
    let profile = builder.push_value(OpenProfile::new(trimmed)).unwrap();
    let graph = builder.finish(vec![profile]).unwrap();

    assert!(matches!(
        graph.get(profile),
        Some(GeometryNode::OpenProfile(OpenProfile { path })) if *path == trimmed
    ));
    assert_eq!(graph.get(profile).unwrap().references(), vec![trimmed]);
}

#[test]
fn open_profile_rejects_a_three_dimensional_curve() {
    let mut builder = GeometryGraphBuilder::new();
    let curve = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Vec3::ZERO,
            direction: Vec3::X,
        })))
        .unwrap();
    let error = builder.push_value(OpenProfile::new(curve)).unwrap_err();

    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference,
            expected: "bounded open curve2",
            actual: "curve3",
        } if reference == curve
    ));
}

#[test]
fn open_profile_cannot_be_used_as_an_area_for_extrusion() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        })))
        .unwrap();
    let profile = builder.push_value(OpenProfile::new(path)).unwrap();
    let error = builder
        .push_value(SolidOperation::Extrusion {
            profile,
            direction: Vec3::Z,
            depth: 1.0,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference,
            expected: "profile",
            actual: "open-profile",
        } if reference == profile
    ));
}

#[test]
fn open_profile_rejects_non_curve_payloads() {
    let mut builder = GeometryGraphBuilder::new();
    let point = builder.push(GeometryNode::Point2(Vec2::Y)).unwrap();
    let error = builder.push_value(OpenProfile::new(point)).unwrap_err();

    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference,
            expected: "bounded open curve2",
            actual: "point2",
        } if reference == point
    ));
}
