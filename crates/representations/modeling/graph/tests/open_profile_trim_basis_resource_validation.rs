use axiolid_core::Vec2;
use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraphBuilder, OpenProfile, Transition, TrimSelector,
    TrimmingPreference,
};

#[test]
fn shared_trim_basis_dag_is_validated_in_reachable_graph_time() {
    let mut builder = GeometryGraphBuilder::new();
    let mut basis = builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap();

    for _ in 0..40 {
        basis = builder
            .push_value(CurveRelation::Composite {
                segments: vec![
                    CurveSegment {
                        transition: Transition::Continuous,
                        same_sense: true,
                        curve: basis,
                    },
                    CurveSegment {
                        transition: Transition::Continuous,
                        same_sense: true,
                        curve: basis,
                    },
                ],
            })
            .unwrap();
    }

    let trimmed = builder
        .push_value(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(1.0)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        })
        .unwrap();
    builder.push_value(OpenProfile::new(trimmed)).unwrap();
}
