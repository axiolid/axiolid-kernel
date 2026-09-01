use axiolid_core::Vec2;
use axiolid_curve::{BSplineCurve2, Curve2, KnotSpec, Line2, Polyline2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraphBuilder, GeometryNode, GraphError, OpenProfile,
    Transition, TrimSelector, TrimmingPreference,
};

fn assert_rejected(curve: Curve2) {
    let mut builder = GeometryGraphBuilder::new();
    let curve = builder.push(GeometryNode::Curve2(curve)).unwrap();
    let error = builder.push_value(OpenProfile::new(curve)).unwrap_err();
    assert_invalid_open_reference(error, curve, "curve2");
}

fn assert_invalid_open_reference(
    error: GraphError,
    reference: axiolid_model::NodeId,
    actual: &'static str,
) {
    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference: rejected,
            expected: "bounded open curve2",
            actual: rejected_actual,
        } if rejected == reference && rejected_actual == actual
    ));
}

fn line() -> Curve2 {
    Curve2::Line(Line2 {
        origin: Vec2::ZERO,
        direction: Vec2::X,
    })
}

fn spline(closed: bool) -> Curve2 {
    Curve2::BSpline(BSplineCurve2 {
        degree: 1,
        control_points: vec![Vec2::ZERO, Vec2::X],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed,
        self_intersect: None,
        knot_spec: KnotSpec::QuasiUniform,
    })
}

fn trimmed_line(builder: &mut GeometryGraphBuilder, start: f64, end: f64) -> axiolid_model::NodeId {
    let basis = builder.push_value(line()).unwrap();
    builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(start)],
            end: vec![TrimSelector::Parameter(end)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        })
        .unwrap()
}

#[test]
fn open_profile_rejects_an_unbounded_line() {
    assert_rejected(line());
}

#[test]
fn open_profile_rejects_an_explicitly_closed_polyline() {
    assert_rejected(Curve2::Polyline(Polyline2 {
        points: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
        closed: true,
    }));
}

#[test]
fn open_profile_accepts_a_source_open_bspline() {
    let mut builder = GeometryGraphBuilder::new();
    let curve = builder.push_value(spline(false)).unwrap();
    assert!(builder.push_value(OpenProfile::new(curve)).is_ok());
}

#[test]
fn open_profile_rejects_a_source_closed_bspline() {
    assert_rejected(spline(true));
}

#[test]
fn open_profile_rejects_a_trimmed_curve_without_both_end_selectors() {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder.push_value(line()).unwrap();
    let trimmed = builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: Vec::new(),
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        })
        .unwrap();
    let error = builder.push_value(OpenProfile::new(trimmed)).unwrap_err();
    assert_invalid_open_reference(error, trimmed, "curve-relation");
}

#[test]
fn open_profile_accepts_a_non_empty_composite_of_bounded_open_segments() {
    let mut builder = GeometryGraphBuilder::new();
    let first = trimmed_line(&mut builder, 0.0, 1.0);
    let second = trimmed_line(&mut builder, 1.0, 2.0);
    let composite = builder
        .push_value(CurveRelation::Composite {
            segments: vec![
                CurveSegment {
                    curve: first,
                    same_sense: true,
                    transition: Transition::Discontinuous,
                },
                CurveSegment {
                    curve: second,
                    same_sense: true,
                    transition: Transition::Continuous,
                },
            ],
        })
        .unwrap();

    assert!(builder.push_value(OpenProfile::new(composite)).is_ok());
}

#[test]
fn open_profile_rejects_an_empty_composite() {
    let mut builder = GeometryGraphBuilder::new();
    let composite = builder
        .push_value(CurveRelation::Composite {
            segments: Vec::new(),
        })
        .unwrap();
    let error = builder.push_value(OpenProfile::new(composite)).unwrap_err();
    assert_invalid_open_reference(error, composite, "curve-relation");
}
