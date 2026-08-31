use axiolid_brep::{ExactBRepBuilder, ExactBRepError};
use axiolid_core::{Frame3, Interval, Point2, Point3, Vec2, Vec3};
use axiolid_curve::{Curve2, Curve3, Line2, Line3};
use axiolid_surface::{Plane, Surface};
use axiolid_topology::{Edge, EdgeUse, Face, FaceBound, Loop, Orientation, Vertex};

fn builder_with_square(omit_pcurve: bool, edge_interval: Interval) -> ExactBRepBuilder {
    let mut result = ExactBRepBuilder::default();
    let curve3 = result.add_curve3(Curve3::Line(Line3 {
        origin: Point3::ZERO,
        direction: Vec3::X,
    }));
    let curve2 = result.add_curve2(Curve2::Line(Line2 {
        origin: Point2::ZERO,
        direction: Vec2::X,
    }));
    let surface = result.add_surface(Surface::Plane(Plane {
        frame: Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        },
    }));

    let topology = result.topology_mut();
    let vertices: Vec<_> = (0..4)
        .map(|index| {
            topology.add_vertex(Vertex {
                position: Point3::new(index as f64, 0.0, 0.0),
            })
        })
        .collect();
    let edges: Vec<_> = (0..4)
        .map(|index| {
            topology.add_edge(Edge {
                start: vertices[index],
                end: vertices[(index + 1) % 4],
                curve: Some(curve3),
            })
        })
        .collect();
    let loop_id = topology.add_loop(Loop {
        edges: edges
            .iter()
            .map(|&edge| EdgeUse {
                edge,
                orientation: Orientation::Forward,
                pcurve: (!omit_pcurve).then_some(curve2),
            })
            .collect(),
    });
    topology.add_face(Face {
        surface: Some(surface),
        bounds: vec![FaceBound {
            loop_id,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });
    for edge in edges {
        result.set_edge_interval(edge, edge_interval);
    }
    if !omit_pcurve {
        for use_index in 0..4 {
            result.set_pcurve_interval(loop_id, use_index, Interval::UNIT);
        }
    }
    result
}

#[test]
fn exact_result_preserves_typed_analytic_supports_and_spans() {
    let result = builder_with_square(false, Interval::UNIT)
        .finish()
        .expect("all exact supports and intervals are present");

    assert_eq!(result.curves3().len(), 1);
    assert_eq!(result.curves2().len(), 1);
    assert_eq!(result.surfaces().len(), 1);
    assert_eq!(result.topology().faces().len(), 1);
    let edge = result
        .topology()
        .edge_id_at(0)
        .expect("square has a first edge");
    assert_eq!(result.edge_interval(edge), Some(Interval::UNIT));
}

#[test]
fn exact_result_refuses_missing_pcurve_instead_of_inverting_or_tessellating() {
    let error = builder_with_square(true, Interval::UNIT)
        .finish()
        .expect_err("exact trimmed face requires a pcurve per edge use");
    assert!(matches!(error, ExactBRepError::MissingPcurve { .. }));
}

#[test]
fn exact_result_refuses_zero_or_non_finite_support_spans() {
    for interval in [Interval::new(2.0, 2.0), Interval::new(f64::NAN, 1.0)] {
        let error = builder_with_square(false, interval)
            .finish()
            .expect_err("a bounded exact edge needs a finite non-zero native span");
        assert!(matches!(error, ExactBRepError::InvalidEdgeInterval { .. }));
    }
}
