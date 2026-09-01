use axiolid_core::{Transform3, Vec2, Vec3};
use axiolid_curve::{BSplineCurve2, Curve2, KnotSpec, Polyline2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraphBuilder, GeometryNode, GraphError, Instance,
    OpenProfile, Transition, TrimSelector, TrimmingPreference,
};

fn assert_curve_rejected(curve: Curve2) {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder.push_value(curve).unwrap();
    let error = builder.push_value(OpenProfile::new(path)).unwrap_err();
    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference,
            expected: "bounded open curve2",
            actual: "curve2",
        } if reference == path
    ));
}

fn valid_bspline() -> BSplineCurve2 {
    BSplineCurve2 {
        degree: 1,
        control_points: vec![Vec2::ZERO, Vec2::X],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        closed: false,
        self_intersect: None,
        knot_spec: KnotSpec::Unspecified,
    }
}

#[test]
fn open_profile_rejects_non_finite_and_explicitly_self_closing_polylines() {
    assert_curve_rejected(Curve2::Polyline(Polyline2 {
        points: vec![Vec2::ZERO, Vec2::new(f64::NAN, 1.0)],
        closed: false,
    }));
    assert_curve_rejected(Curve2::Polyline(Polyline2 {
        points: vec![Vec2::ZERO, Vec2::X, Vec2::ZERO],
        closed: false,
    }));
}

#[test]
fn open_profile_rejects_non_finite_bspline_data() {
    let mut bad_control = valid_bspline();
    bad_control.control_points[1].x = f64::NAN;
    let mut bad_knot = valid_bspline();
    bad_knot.knots[1] = f64::INFINITY;
    let mut bad_weight = valid_bspline();
    bad_weight.weights = Some(vec![1.0, f64::NAN]);

    for curve in [bad_control, bad_knot, bad_weight] {
        assert_curve_rejected(Curve2::BSpline(curve));
    }
}

#[test]
fn open_profile_rejects_structurally_invalid_bspline_data() {
    let mut zero_degree = valid_bspline();
    zero_degree.degree = 0;
    let mut excessive_degree = valid_bspline();
    excessive_degree.degree = 2;
    let mut unordered_knots = valid_bspline();
    unordered_knots.knots = vec![1.0, 0.0];
    let mut mismatched_multiplicities = valid_bspline();
    mismatched_multiplicities.multiplicities = vec![2];
    let mut invalid_multiplicity_sum = valid_bspline();
    invalid_multiplicity_sum.multiplicities = vec![1, 1];
    let mut mismatched_weights = valid_bspline();
    mismatched_weights.weights = Some(vec![1.0]);
    let mut non_positive_weights = valid_bspline();
    non_positive_weights.weights = Some(vec![1.0, 0.0]);

    for curve in [
        zero_degree,
        excessive_degree,
        unordered_knots,
        mismatched_multiplicities,
        invalid_multiplicity_sum,
        mismatched_weights,
        non_positive_weights,
    ] {
        assert_curve_rejected(Curve2::BSpline(curve));
    }
}

#[test]
fn open_profile_rejects_a_non_finite_offset_distance() {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    let offset = builder
        .push_value(CurveRelation::Offset {
            basis,
            distance: f64::NAN,
            reference_direction: None,
        })
        .unwrap();

    let error = builder.push_value(OpenProfile::new(offset)).unwrap_err();
    assert!(matches!(
        error,
        GraphError::InvalidReferenceType { reference, .. } if reference == offset
    ));
}

#[test]
fn open_profile_rejects_a_non_finite_instance_transform() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    let instance = builder
        .push(GeometryNode::Instance(Instance {
            source: path,
            transform: Transform3::from_translation(Vec3::new(f64::NAN, 0.0, 0.0)),
        }))
        .unwrap();

    let error = builder.push_value(OpenProfile::new(instance)).unwrap_err();
    assert!(matches!(
        error,
        GraphError::InvalidReferenceType { reference, .. } if reference == instance
    ));
}

fn wrap_in_finite_trim(
    builder: &mut GeometryGraphBuilder,
    basis: axiolid_model::NodeId,
) -> axiolid_model::NodeId {
    builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(1.0)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        })
        .unwrap()
}

#[test]
fn open_profile_rejects_malformed_atomic_curves_hidden_under_a_trim() {
    let mut invalid_spline = valid_bspline();
    invalid_spline.weights = Some(vec![1.0, 0.0]);
    for curve in [
        Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::new(f64::NAN, 1.0)],
            closed: false,
        }),
        Curve2::BSpline(invalid_spline),
    ] {
        let mut builder = GeometryGraphBuilder::new();
        let basis = builder.push_value(curve).unwrap();
        let trimmed = wrap_in_finite_trim(&mut builder, basis);
        let error = builder.push_value(OpenProfile::new(trimmed)).unwrap_err();
        assert!(matches!(
            error,
            GraphError::InvalidReferenceType { reference, .. } if reference == trimmed
        ));
    }
}

#[test]
fn open_profile_rejects_a_non_finite_offset_hidden_under_a_trim() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    let offset = builder
        .push_value(CurveRelation::Offset {
            basis: path,
            distance: f64::NAN,
            reference_direction: None,
        })
        .unwrap();
    let trimmed = wrap_in_finite_trim(&mut builder, offset);
    assert!(builder.push_value(OpenProfile::new(trimmed)).is_err());
}

#[test]
fn open_profile_rejects_a_non_finite_instance_hidden_under_a_trim() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    let instance = builder
        .push(GeometryNode::Instance(Instance {
            source: path,
            transform: Transform3::from_translation(Vec3::new(f64::NAN, 0.0, 0.0)),
        }))
        .unwrap();
    let trimmed = wrap_in_finite_trim(&mut builder, instance);
    assert!(builder.push_value(OpenProfile::new(trimmed)).is_err());
}

#[test]
fn open_profile_rejects_a_non_finite_instance_hidden_in_a_composite() {
    let mut builder = GeometryGraphBuilder::new();
    let path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    let instance = builder
        .push(GeometryNode::Instance(Instance {
            source: path,
            transform: Transform3::from_translation(Vec3::new(f64::NAN, 0.0, 0.0)),
        }))
        .unwrap();
    let composite = builder
        .push_value(CurveRelation::Composite {
            segments: vec![CurveSegment {
                transition: Transition::Continuous,
                same_sense: true,
                curve: instance,
            }],
        })
        .unwrap();
    assert!(builder.push_value(OpenProfile::new(composite)).is_err());
}
