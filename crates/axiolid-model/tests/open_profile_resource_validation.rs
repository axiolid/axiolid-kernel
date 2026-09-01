use axiolid_core::{Vec2, Vec3};
use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{CurveRelation, CurveSegment, GeometryGraphBuilder, OpenProfile, Transition};

fn source_open_polyline(builder: &mut GeometryGraphBuilder) -> axiolid_model::NodeId {
    builder
        .push_value(Curve2::Polyline(Polyline2 {
            points: vec![Vec2::ZERO, Vec2::X],
            closed: false,
        }))
        .unwrap()
}

#[test]
fn open_profile_rejects_direct_offset_reference_directions() {
    for direction in [Vec3::Z, Vec3::new(f64::NAN, 0.0, 1.0)] {
        let mut builder = GeometryGraphBuilder::new();
        let path = source_open_polyline(&mut builder);
        let offset = builder
            .push_value(CurveRelation::Offset {
                basis: path,
                distance: 1.0,
                reference_direction: Some(direction),
            })
            .unwrap();
        assert!(builder.push_value(OpenProfile::new(offset)).is_err());
    }
}

#[test]
fn shared_composite_dag_is_validated_in_reachable_graph_time() {
    let mut builder = GeometryGraphBuilder::new();
    let mut path = source_open_polyline(&mut builder);

    for _ in 0..40 {
        path = builder
            .push_value(CurveRelation::Composite {
                segments: vec![
                    CurveSegment {
                        transition: Transition::Continuous,
                        same_sense: true,
                        curve: path,
                    },
                    CurveSegment {
                        transition: Transition::Continuous,
                        same_sense: true,
                        curve: path,
                    },
                ],
            })
            .unwrap();
    }

    builder.push_value(OpenProfile::new(path)).unwrap();
}
