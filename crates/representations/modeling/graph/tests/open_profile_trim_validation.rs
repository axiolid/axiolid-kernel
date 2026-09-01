use axiolid_core::{Transform3, Vec2, Vec3};
use axiolid_curve::{Curve2, Line2, Polyline2};
use axiolid_model::{
    CurveRelation, GeometryGraphBuilder, GraphError, Instance, OpenProfile, TrimSelector,
    TrimmingPreference,
};

fn line() -> Curve2 {
    Curve2::Line(Line2 {
        origin: Vec2::ZERO,
        direction: Vec2::X,
    })
}

fn assert_trim_rejected(
    start: Vec<TrimSelector>,
    end: Vec<TrimSelector>,
    preference: TrimmingPreference,
) {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder.push_value(line()).unwrap();
    let trimmed = builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start,
            end,
            sense_agreement: true,
            preference,
        })
        .unwrap();
    let error = builder.push_value(OpenProfile::new(trimmed)).unwrap_err();
    assert!(matches!(
        error,
        GraphError::InvalidReferenceType {
            reference,
            expected: "bounded open curve2",
            actual: "curve-relation",
        } if reference == trimmed
    ));
}

#[test]
fn open_profile_rejects_a_missing_trim_endpoint_with_unspecified_preference() {
    assert_trim_rejected(
        vec![TrimSelector::Parameter(0.0)],
        Vec::new(),
        TrimmingPreference::Unspecified,
    );
}

#[test]
fn open_profile_rejects_three_dimensional_trim_selectors() {
    assert_trim_rejected(
        vec![TrimSelector::Point3(Vec3::ZERO)],
        vec![TrimSelector::Point3(Vec3::X)],
        TrimmingPreference::Cartesian,
    );
}

#[test]
fn open_profile_rejects_point3_even_when_a_preferred_point2_is_present() {
    assert_trim_rejected(
        vec![
            TrimSelector::Point2(Vec2::ZERO),
            TrimSelector::Point3(Vec3::ZERO),
        ],
        vec![TrimSelector::Point2(Vec2::X), TrimSelector::Point3(Vec3::X)],
        TrimmingPreference::Cartesian,
    );
}

#[test]
fn open_profile_rejects_non_finite_parameter_selectors() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_trim_rejected(
            vec![TrimSelector::Parameter(value)],
            vec![TrimSelector::Parameter(1.0)],
            TrimmingPreference::Parameter,
        );
    }
}

#[test]
fn open_profile_rejects_non_finite_point_selectors() {
    assert_trim_rejected(
        vec![TrimSelector::Point2(Vec2::new(f64::NAN, 0.0))],
        vec![TrimSelector::Point2(Vec2::X)],
        TrimmingPreference::Cartesian,
    );
}

#[test]
fn open_profile_rejects_exactly_equal_parameter_endpoints() {
    assert_trim_rejected(
        vec![TrimSelector::Parameter(-0.0)],
        vec![TrimSelector::Parameter(0.0)],
        TrimmingPreference::Parameter,
    );
}

#[test]
fn open_profile_rejects_exactly_equal_point_endpoints() {
    assert_trim_rejected(
        vec![TrimSelector::Point2(Vec2::new(1.0, 2.0))],
        vec![TrimSelector::Point2(Vec2::new(1.0, 2.0))],
        TrimmingPreference::Cartesian,
    );
}

#[test]
fn open_profile_requires_parameter_selectors_when_parameter_is_preferred() {
    assert_trim_rejected(
        vec![TrimSelector::Point2(Vec2::ZERO)],
        vec![TrimSelector::Point2(Vec2::X)],
        TrimmingPreference::Parameter,
    );
}

#[test]
fn open_profile_requires_point_selectors_when_cartesian_is_preferred() {
    assert_trim_rejected(
        vec![TrimSelector::Parameter(0.0)],
        vec![TrimSelector::Parameter(1.0)],
        TrimmingPreference::Cartesian,
    );
}

#[test]
fn open_profile_accepts_distinct_finite_cartesian_endpoints() {
    let mut builder = GeometryGraphBuilder::new();
    let basis = builder.push_value(line()).unwrap();
    let trimmed = builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Point2(Vec2::ZERO)],
            end: vec![TrimSelector::Point2(Vec2::X)],
            sense_agreement: true,
            preference: TrimmingPreference::Cartesian,
        })
        .unwrap();

    assert!(builder.push_value(OpenProfile::new(trimmed)).is_ok());
}

#[test]
fn open_profile_rejects_malformed_selectors_hidden_in_a_nested_trim() {
    for (start, end, preference) in [
        (
            vec![TrimSelector::Parameter(f64::NAN)],
            vec![TrimSelector::Parameter(1.0)],
            TrimmingPreference::Parameter,
        ),
        (
            vec![TrimSelector::Point3(Vec3::ZERO)],
            vec![TrimSelector::Point3(Vec3::X)],
            TrimmingPreference::Unspecified,
        ),
        (
            vec![TrimSelector::Parameter(0.0)],
            Vec::new(),
            TrimmingPreference::Unspecified,
        ),
    ] {
        let mut builder = GeometryGraphBuilder::new();
        let line = builder.push_value(line()).unwrap();
        let malformed_basis = builder
            .push_value(CurveRelation::Trimmed {
                basis: line,
                start,
                end,
                sense_agreement: true,
                preference,
            })
            .unwrap();
        let outer_trim = builder
            .push_value(CurveRelation::Trimmed {
                basis: malformed_basis,
                start: vec![TrimSelector::Parameter(0.0)],
                end: vec![TrimSelector::Parameter(1.0)],
                sense_agreement: true,
                preference: TrimmingPreference::Parameter,
            })
            .unwrap();

        let error = builder
            .push_value(OpenProfile::new(outer_trim))
            .unwrap_err();
        assert!(matches!(
            error,
            GraphError::InvalidReferenceType {
                reference,
                expected: "bounded open curve2",
                actual: "curve-relation",
            } if reference == outer_trim
        ));
    }
}

#[test]
fn deep_instance_chain_is_validated_iteratively() {
    let mut builder = GeometryGraphBuilder::new();
    let mut path = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();
    for _ in 0..4_096 {
        path = builder
            .push_value(Instance {
                source: path,
                transform: Transform3::IDENTITY,
            })
            .unwrap();
    }

    assert!(builder.push_value(OpenProfile::new(path)).is_ok());
}
