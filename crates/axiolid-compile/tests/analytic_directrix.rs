//! Analytic directrices and parameter-range trimming, end to end.
//!
//! A polyline directrix arrives pre-sampled, so its point count is the
//! caller's choice. An analytic curve does not: the compiler decides where
//! to sample, and these tests pin that the decision is driven by the chord
//! budget and by the curve's own parameterisation.

use axiolid_boolmesh::BoolmeshBoolean;
use axiolid_compile::ScalarCompiler;
use axiolid_core::{Frame3, Point3, Scalar, Tolerance, Vec3};
use axiolid_curve::{Curve3, Line3};
use axiolid_kernel::{ExecutionOptions, GeometryCompiler};
use axiolid_measure::volume_properties;
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryGraphBuilder, GeometryNode, SolidOperation, Transition,
    TrimSelector, TrimmingPreference,
};
use axiolid_profile::{Profile, RectangleProfile};

fn compiler() -> ScalarCompiler<BoolmeshBoolean> {
    ScalarCompiler::new(BoolmeshBoolean::new())
}

fn options() -> ExecutionOptions {
    ExecutionOptions::new(Tolerance::MILLIMETRE)
}

fn rect(x: Scalar, y: Scalar) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness: None,
        outer_radius: None,
        inner_radius: None,
    })
}

fn frame() -> Frame3 {
    Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

/// Build a fixed-reference sweep of `profile` along `curve`, optionally
/// trimmed, and return the compiled result.
fn sweep_along(
    curve: axiolid_curve::Curve3,
    range: Option<(Scalar, Scalar)>,
) -> axiolid_kernel::GeomResult<axiolid_mesh::TriMesh> {
    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(1.0, 2.0))).unwrap();
    let directrix = b.push(GeometryNode::Curve3(curve)).unwrap();
    let swept = b
        .push(GeometryNode::SolidOperation(
            SolidOperation::FixedReferenceSweep {
                profile,
                directrix,
                reference_direction: Vec3::Z,
                parameter_range: range,
            },
        ))
        .unwrap();
    let graph = b.finish(vec![swept]).unwrap();
    compiler().compile(&graph, swept, &options())
}

/// A circular directrix sweeps a torus whose volume Pappus gives exactly.
///
/// Nothing in the graph says how many points the circle is worth: the
/// compiler chooses them from the chord budget. The closed form is what
/// makes that choice checkable, and it also rules out the previous
/// behaviour, which refused the operation outright.
#[test]
fn a_circular_directrix_sweeps_a_torus() {
    let radius = 5.0;
    let mesh = sweep_along(
        axiolid_curve::Curve3::Circle(axiolid_curve::Circle3 {
            frame: frame(),
            radius,
        }),
        None,
    )
    .expect("an analytic circle is a valid directrix");

    let volume = volume_properties(&mesh, Tolerance::MILLIMETRE)
        .expect("a swept solid must be closed and two-manifold")
        .signed_volume
        .abs();
    // Pappus: section area times the centroid's travel.
    let exact = 1.0 * 2.0 * core::f64::consts::TAU * radius;
    let ratio = volume / exact;
    assert!(
        (0.99..=1.0).contains(&ratio),
        "swept volume {volume} against exact {exact} (ratio {ratio})"
    );
}

/// A tighter chord budget samples the same curve more finely.
///
/// This is the property that distinguishes adaptive sampling from a fixed
/// count: the mesh must actually respond to the tolerance. It also pins the
/// direction -- finer tolerance, closer to the analytic value -- so a
/// sampler that ignored the budget would fail even if it were dense.
#[test]
fn a_tighter_tolerance_samples_more_finely() {
    let radius = 5.0;
    let build = |tolerance: Tolerance| {
        let mut b = GeometryGraphBuilder::new();
        let profile = b.push(GeometryNode::Profile(rect(1.0, 2.0))).unwrap();
        let directrix = b
            .push(GeometryNode::Curve3(axiolid_curve::Curve3::Circle(
                axiolid_curve::Circle3 {
                    frame: frame(),
                    radius,
                },
            )))
            .unwrap();
        let swept = b
            .push(GeometryNode::SolidOperation(
                SolidOperation::FixedReferenceSweep {
                    profile,
                    directrix,
                    reference_direction: Vec3::Z,
                    parameter_range: None,
                },
            ))
            .unwrap();
        let graph = b.finish(vec![swept]).unwrap();
        compiler()
            .compile(&graph, swept, &ExecutionOptions::new(tolerance))
            .expect("compiles at either tolerance")
    };

    let coarse = build(Tolerance::new(1e-2, 1e-9).expect("coarse"));
    let fine = build(Tolerance::new(1e-5, 1e-9).expect("fine"));
    assert!(
        fine.positions.len() > coarse.positions.len(),
        "a tighter chord budget must sample more finely: {} vs {}",
        fine.positions.len(),
        coarse.positions.len()
    );

    let exact = 1.0 * 2.0 * core::f64::consts::TAU * radius;
    let v = |m: &axiolid_mesh::TriMesh| {
        volume_properties(m, Tolerance::MILLIMETRE)
            .expect("closed")
            .signed_volume
            .abs()
    };
    // Both inscribe the true torus, and the finer one inscribes it closer.
    assert!(
        v(&fine) > v(&coarse),
        "finer sampling must lose less volume"
    );
    assert!(
        v(&fine) <= exact,
        "a chordal sweep cannot exceed the exact volume"
    );
}

/// Half the parameter range sweeps half the solid.
///
/// The trim is applied in the curve's OWN parameterisation, so for a
/// circle -- parameterised over a full turn -- half the domain is half the
/// arc and therefore half the volume. An implementation that sliced the
/// sampled points by index would pass this only by accident, and the
/// domain-mismatch test below is what separates the two.
#[test]
fn a_half_parameter_range_sweeps_half_the_solid() {
    let radius = 5.0;
    let circle = || {
        axiolid_curve::Curve3::Circle(axiolid_curve::Circle3 {
            frame: frame(),
            radius,
        })
    };
    let whole = sweep_along(circle(), None).expect("untrimmed");
    let half = sweep_along(circle(), Some((0.0, core::f64::consts::PI)))
        .expect("a half-turn range is inside the circle's domain");

    let v = |m: &axiolid_mesh::TriMesh| {
        volume_properties(m, Tolerance::MILLIMETRE)
            .expect("closed")
            .signed_volume
            .abs()
    };
    let ratio = v(&half) / v(&whole);
    assert!(
        (0.49..=0.51).contains(&ratio),
        "a half-turn trim swept {ratio} of the full solid"
    );
}

/// A range outside the curve's domain is refused, not clamped silently.
///
/// Extrapolating a bounded curve invents geometry. The circle's domain is
/// one turn, so asking for two is a modelling error worth naming rather
/// than quietly sweeping the turn that does exist.
#[test]
fn a_range_outside_the_domain_is_refused() {
    let out_of_range = sweep_along(
        axiolid_curve::Curve3::Circle(axiolid_curve::Circle3 {
            frame: frame(),
            radius: 5.0,
        }),
        Some((0.0, 2.0 * core::f64::consts::TAU)),
    );
    assert!(
        out_of_range.is_err(),
        "a range beyond the curve's domain must be refused, not clamped"
    );
}

/// An empty range is refused: it names no arc at all.
#[test]
fn an_empty_range_is_refused() {
    let empty = sweep_along(
        axiolid_curve::Curve3::Circle(axiolid_curve::Circle3 {
            frame: frame(),
            radius: 5.0,
        }),
        Some((1.0, 1.0)),
    );
    // Assert the SPECIFIC diagnosis, not merely that something failed. An
    // empty range degenerates into a zero-length path, which downstream
    // code rejects for its own reasons; without pinning the message this
    // test would pass even if the range check were removed entirely.
    match empty {
        Err(axiolid_kernel::GeomError::Degenerate(message)) => {
            assert!(
                message.contains("empty"),
                "expected the range to be named as empty, got: {message}"
            );
        }
        other => panic!("expected a Degenerate empty-range error, got {other:?}"),
    }
}

/// A polyline range in the WRONG units is refused rather than silently
/// collapsing the path.
///
/// A polyline's parameter is one unit per segment, so a caller passing a
/// normalised (0, 1) range for a many-segment path is asking for the first
/// edge only. That is almost never the intent, and sweeping it would
/// produce a solid a fraction of the requested length.
#[test]
fn a_normalised_range_on_a_polyline_is_refused() {
    let path = axiolid_curve::Curve3::Polyline(axiolid_curve::Polyline3 {
        points: vec![
            Point3::ZERO,
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(0.0, 0.0, 3.0),
        ],
        closed: false,
    });
    assert!(
        sweep_along(path, Some((0.0, 1.0))).is_err(),
        "a normalised range on a 3-segment polyline must be refused"
    );
}

#[test]
fn swept_disk_accepts_trimmed_composite_directrix_with_segment_sense() {
    let mut builder = GeometryGraphBuilder::new();
    let first_line = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Point3::ZERO,
            direction: Vec3::X,
        })))
        .unwrap();
    let first_trim = builder
        .push(GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis: first_line,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(2.0)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }))
        .unwrap();

    let second_line = builder
        .push(GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Point3::new(4.0, 0.0, 0.0),
            direction: -Vec3::X,
        })))
        .unwrap();
    let second_trim = builder
        .push(GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis: second_line,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(2.0)],
            sense_agreement: true,
            preference: TrimmingPreference::Parameter,
        }))
        .unwrap();
    let composite = builder
        .push(GeometryNode::CurveRelation(CurveRelation::Composite {
            segments: vec![
                CurveSegment {
                    curve: first_trim,
                    transition: Transition::Continuous,
                    same_sense: true,
                },
                CurveSegment {
                    curve: second_trim,
                    transition: Transition::Continuous,
                    same_sense: false,
                },
            ],
        }))
        .unwrap();
    let sweep = builder
        .push(GeometryNode::SolidOperation(SolidOperation::SweptDisk {
            directrix: composite,
            radius: 0.1,
            inner_radius: None,
            parameter_range: None,
            fillet_radius: None,
        }))
        .unwrap();
    let graph = builder.finish(vec![sweep]).unwrap();

    let mesh = compiler()
        .compile(&graph, sweep, &options())
        .expect("composite relation directrix");
    assert!(!mesh.indices.is_empty());
    assert!(mesh.positions.iter().all(|p| p.is_finite()));
    let max_x = mesh.positions.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    let max_y = mesh.positions.iter().map(|p| p.y).fold(f64::MIN, f64::max);
    assert!(max_x >= 4.0, "x extent {max_x}");
    assert!(max_y > 0.09, "y extent {max_y}");
}
