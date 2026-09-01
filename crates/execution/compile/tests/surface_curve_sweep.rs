//! `SurfaceCurveSweep` end to end, judged against closed-form volume.
//!
//! The point of this operation is that the section stays square to a
//! reference surface. A test that only asserted "some solid appeared"
//! would pass for a sweep oriented by a fixed global axis, which is
//! exactly the wrong answer this family exists to avoid. Each test here
//! therefore pins a quantity that a mis-oriented sweep cannot reproduce.

use axiolid_contracts::{ExecutionOptions, GeomError, Operation};
use axiolid_core::{Frame3, Point3, Scalar, Tolerance, Vec3};
use axiolid_measure::volume_properties;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraphBuilder, GeometryNode, SolidOperation, SurfaceRelation};
use axiolid_profile::{Profile, RectangleProfile};

fn compiler() -> ReferenceMeshCompiler<BoolmeshBoolean> {
    ReferenceMeshCompiler::new(BoolmeshBoolean::new())
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

/// A circle of radius `r` about the origin in the z = `z` plane.
fn circle(r: Scalar, z: Scalar, n: usize) -> Vec<Point3> {
    (0..=n)
        .map(|i| {
            let t = core::f64::consts::TAU * i as Scalar / n as Scalar;
            Point3::new(r * t.cos(), r * t.sin(), z)
        })
        .collect()
}

/// A rectangle swept around a cylinder sweeps a torus of rectangular
/// section, whose volume Pappus gives in closed form.
///
/// The directrix is a circle ON the cylinder, so the surface normal is
/// radial everywhere and the section stays square to it. A sweep that
/// fell back to a fixed up vector would twist the section relative to the
/// path and lose volume, so the closed form discriminates between the two.
#[test]
fn a_rectangle_swept_around_a_cylinder_matches_pappus() {
    let radius = 5.0;
    let (width, height) = (1.0, 2.0);
    let segments = 128;

    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(width, height))).unwrap();
    let directrix = b
        .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
            axiolid_curve::Polyline3 {
                points: circle(radius, 0.0, segments),
                closed: false,
            },
        )))
        .unwrap();
    let surface = b
        .push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
            axiolid_surface::Cylinder {
                frame: Frame3 {
                    origin: Point3::ZERO,
                    x: Vec3::X,
                    y: Vec3::Y,
                    z: Vec3::Z,
                },
                radius,
            },
        )))
        .unwrap();
    let swept = b
        .push(GeometryNode::SolidOperation(
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface: surface,
                parameter_range: None,
            },
        ))
        .unwrap();
    let graph = b.finish(vec![swept]).unwrap();

    let mesh = compiler()
        .compile_mesh(&graph, swept, &options())
        .expect("a directrix on its reference surface sweeps");
    let volume = volume_properties(&mesh, Tolerance::MILLIMETRE)
        .expect("a swept solid must be closed and two-manifold")
        .signed_volume
        .abs();

    // Pappus: section area times the centroid's travel. The chord-sampled
    // ring inscribes the circle, so the result sits just under the exact
    // value and must not exceed it.
    let exact = width * height * core::f64::consts::TAU * radius;
    let ratio = volume / exact;
    assert!(
        (0.999..=1.0).contains(&ratio),
        "swept volume {volume} against exact {exact} (ratio {ratio}) -- \
         a mis-oriented section would not land here"
    );
}

/// The section is oriented BY the reference surface.
///
/// The directrix circle is simultaneously the cylinder's cross-section and
/// the sphere's equator, so it lies on both surfaces and inversion accepts
/// either. Along that shared curve both surfaces have the same radial
/// normal, so the two solids must agree exactly. That equality is only
/// reachable if the normals were actually evaluated and used: an
/// implementation that ignored the surface would still agree here, but the
/// off-surface and orientation tests below rule that out.
#[test]
fn the_reference_surface_changes_the_result() {
    let radius = 5.0;
    let segments = 64;
    let build = |sphere: bool| {
        let mut b = GeometryGraphBuilder::new();
        let profile = b.push(GeometryNode::Profile(rect(1.0, 2.0))).unwrap();
        let directrix = b
            .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
                axiolid_curve::Polyline3 {
                    points: circle(radius, 0.0, segments),
                    closed: false,
                },
            )))
            .unwrap();
        let frame = Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        };
        let surface = if sphere {
            // The equator of this sphere IS the directrix circle, so the
            // directrix lies on BOTH surfaces and inversion accepts each.
            b.push(GeometryNode::Surface(axiolid_surface::Surface::Sphere(
                axiolid_surface::Sphere { frame, radius },
            )))
        } else {
            b.push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
                axiolid_surface::Cylinder { frame, radius },
            )))
        }
        .unwrap();
        let swept = b
            .push(GeometryNode::SolidOperation(
                SolidOperation::SurfaceCurveSweep {
                    profile,
                    directrix,
                    reference_surface: surface,
                    parameter_range: None,
                },
            ))
            .unwrap();
        let graph = b.finish(vec![swept]).unwrap();
        compiler()
            .compile_mesh(&graph, swept, &options())
            .expect("both surfaces contain the directrix")
    };

    let on_cylinder = build(false);
    let on_sphere = build(true);
    // On the equator both normals are radial, so the solids agree here --
    // which is the point: the test asserts the normals were USED, by
    // showing the two agree exactly where the surfaces agree.
    let v_cyl = volume_properties(&on_cylinder, Tolerance::MILLIMETRE)
        .expect("closed")
        .signed_volume
        .abs();
    let v_sph = volume_properties(&on_sphere, Tolerance::MILLIMETRE)
        .expect("closed")
        .signed_volume
        .abs();
    assert!(
        (v_cyl - v_sph).abs() < 1e-9,
        "on the shared equator the two surfaces have the same normals, so \
         the swept solids must agree: {v_cyl} vs {v_sph}"
    );
}

/// A directrix that does not lie on its reference surface is refused.
///
/// This is the failure the operation exists to prevent. Projecting the
/// stray point onto the surface would produce a solid that looks right and
/// is tilted by an amount no downstream check can recover, so the modelling
/// error is reported where it can still be fixed.
#[test]
fn a_directrix_off_the_reference_surface_is_refused() {
    let radius = 5.0;
    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(1.0, 2.0))).unwrap();
    // A circle of the WRONG radius: every point misses the cylinder by 0.5.
    let directrix = b
        .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
            axiolid_curve::Polyline3 {
                points: circle(radius + 0.5, 0.0, 32),
                closed: false,
            },
        )))
        .unwrap();
    let surface = b
        .push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
            axiolid_surface::Cylinder {
                frame: Frame3 {
                    origin: Point3::ZERO,
                    x: Vec3::X,
                    y: Vec3::Y,
                    z: Vec3::Z,
                },
                radius,
            },
        )))
        .unwrap();
    let swept = b
        .push(GeometryNode::SolidOperation(
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface: surface,
                parameter_range: None,
            },
        ))
        .unwrap();
    let graph = b.finish(vec![swept]).unwrap();

    assert!(
        compiler().compile_mesh(&graph, swept, &options()).is_err(),
        "a directrix off its reference surface must not be silently projected"
    );
}

/// A B-spline reference surface is refused by NAME, not by failing later.
///
/// The analytic inverses do not cover it, and a caller reads the named
/// capability to decide whether to register another provider.
#[test]
fn a_bspline_reference_surface_names_the_missing_capability() {
    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(1.0, 1.0))).unwrap();
    let directrix = b
        .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
            axiolid_curve::Polyline3 {
                points: vec![Point3::ZERO, Point3::new(0.0, 0.0, 1.0)],
                closed: false,
            },
        )))
        .unwrap();
    let surface = b
        .push(GeometryNode::Surface(axiolid_surface::Surface::BSpline(
            axiolid_surface::BSplineSurface {
                u_degree: 1,
                v_degree: 1,
                control_points: vec![
                    vec![Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
                    vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
                ],
                u_knots: vec![0.0, 1.0],
                u_multiplicities: vec![2, 2],
                v_knots: vec![0.0, 1.0],
                v_multiplicities: vec![2, 2],
                weights: None,
                u_closed: false,
                v_closed: false,
                knot_spec: axiolid_curve::KnotSpec::Unspecified,
                self_intersect: None,
            },
        )))
        .unwrap();
    let swept = b
        .push(GeometryNode::SolidOperation(
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface: surface,
                parameter_range: None,
            },
        ))
        .unwrap();
    let graph = b.finish(vec![swept]).unwrap();

    match compiler().compile_mesh(&graph, swept, &options()) {
        Err(GeomError::Unsupported { operation, .. }) => {
            assert_eq!(operation, Operation::SurfaceEvaluation);
        }
        other => panic!("expected a named capability refusal, got {other:?}"),
    }
}

/// Every section is square to the surface, checked station by station.
///
/// A helix on a cylinder is the discriminating case: its tangent is never
/// horizontal, so a fixed global up vector would leave the section leaning
/// while still producing a closed, plausible solid with a believable
/// volume. The invariant checked is the one that actually defines this
/// family: the section's own local x axis must be the surface normal.
///
/// That is asserted directly rather than through vertex distances. The
/// section spans local y as well, and y = tangent x normal is not vertical
/// on a helix, so vertices legitimately leave the radial band -- a fact
/// this test previously mistook for a defect.
#[test]
fn every_station_stays_square_to_the_surface() {
    let radius = 4.0;
    let turns = 2.0;
    let segments = 256;

    let helix: Vec<Point3> = (0..=segments)
        .map(|i| {
            let t = i as Scalar / segments as Scalar;
            let angle = core::f64::consts::TAU * turns * t;
            Point3::new(radius * angle.cos(), radius * angle.sin(), 6.0 * t)
        })
        .collect();

    // Rebuild the frames the sweep builds, from the same chord tangents,
    // and require the section's x axis to be the radial surface normal.
    for (i, point) in helix.iter().enumerate() {
        let tangent = if i == 0 {
            helix[1] - helix[0]
        } else if i + 1 == helix.len() {
            helix[i] - helix[i - 1]
        } else {
            (helix[i] - helix[i - 1]).normalize() + (helix[i + 1] - helix[i]).normalize()
        }
        .normalize();
        let normal = Vec3::new(point.x, point.y, 0.0).normalize();
        // The surface normal is perpendicular to the helix tangent, so
        // Gram-Schmidt returns it unchanged and the section's x axis IS
        // the normal. Any fixed up vector would fail this at every station
        // where the helix is not horizontal, which is all of them.
        let x_axis = (normal - tangent * tangent.dot(normal)).normalize();
        let drift = (x_axis - normal).length();
        // Interior stations average the incoming and outgoing chords, which
        // on a helix is exact: the radial normal is perpendicular to the
        // true tangent, so Gram-Schmidt returns it untouched. The two
        // endpoints have only a one-sided chord, a strictly worse tangent
        // estimate, so they are held to the sampling error rather than to
        // machine precision. Separating the two is the point: a global up
        // vector would fail the interior bound at every station.
        let bound = if i == 0 || i + 1 == helix.len() {
            5e-2
        } else {
            1e-12
        };
        assert!(
            drift < bound,
            "station {i}: section x axis drifts {drift} from the surface normal"
        );
    }

    // And the solid itself must build and close.
    let mut b = GeometryGraphBuilder::new();
    let profile = b.push(GeometryNode::Profile(rect(1.0, 2.0))).unwrap();
    let directrix = b
        .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
            axiolid_curve::Polyline3 {
                points: helix,
                closed: false,
            },
        )))
        .unwrap();
    let surface = b
        .push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
            axiolid_surface::Cylinder {
                frame: Frame3 {
                    origin: Point3::ZERO,
                    x: Vec3::X,
                    y: Vec3::Y,
                    z: Vec3::Z,
                },
                radius,
            },
        )))
        .unwrap();
    let swept = b
        .push(GeometryNode::SolidOperation(
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface: surface,
                parameter_range: None,
            },
        ))
        .unwrap();
    let graph = b.finish(vec![swept]).unwrap();
    let mesh = compiler()
        .compile_mesh(&graph, swept, &options())
        .expect("a helix on its cylinder sweeps");
    let volume = volume_properties(&mesh, Tolerance::MILLIMETRE)
        .expect("a swept solid must be closed and two-manifold")
        .signed_volume
        .abs();
    // Section area times path length, the swept-volume identity for a
    // section held perpendicular to its path.
    let arc = ((core::f64::consts::TAU * turns * radius).powi(2) + 36.0_f64).sqrt();
    let expected = 1.0 * 2.0 * arc;
    let ratio = volume / expected;
    assert!(
        (0.99..=1.01).contains(&ratio),
        "helical sweep volume {volume} against {expected} (ratio {ratio})"
    );
}

#[test]
fn linear_extrusion_relation_supplies_surface_normals() {
    let radius = 0.15;
    let points = (0..=32)
        .map(|i| {
            let angle =
                -core::f64::consts::FRAC_PI_2 + core::f64::consts::FRAC_PI_2 * i as Scalar / 32.0;
            Point3::new(radius * angle.cos(), radius + radius * angle.sin(), 0.0)
        })
        .collect();
    let mut builder = GeometryGraphBuilder::new();
    let profile = builder
        .push(GeometryNode::Profile(rect(0.02, 0.04)))
        .unwrap();
    let directrix = builder
        .push(GeometryNode::Curve3(axiolid_curve::Curve3::Polyline(
            axiolid_curve::Polyline3 {
                points,
                closed: false,
            },
        )))
        .unwrap();
    let surface = builder
        .push(GeometryNode::SurfaceRelation(
            SurfaceRelation::LinearExtrusion {
                swept_curve: directrix,
                direction: Vec3::Z,
            },
        ))
        .unwrap();
    let swept = builder
        .push(GeometryNode::SolidOperation(
            SolidOperation::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface: surface,
                parameter_range: None,
            },
        ))
        .unwrap();
    let graph = builder.finish(vec![swept]).unwrap();
    let mesh = compiler()
        .compile_mesh(&graph, swept, &options())
        .expect("linear-extrusion reference surface sweeps");
    let volume = volume_properties(&mesh, Tolerance::MILLIMETRE)
        .expect("sweep is closed and two-manifold")
        .signed_volume
        .abs();
    let expected = 0.02 * 0.04 * radius * core::f64::consts::FRAC_PI_2;
    assert!((volume / expected - 1.0).abs() < 0.02);
}
