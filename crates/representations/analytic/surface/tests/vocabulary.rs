//! Contracts `axiolid-surface` owes the crates that consume it.
//!
//! Like `axiolid-curve`, this crate is a value vocabulary. Evaluation, normals,
//! and inversion live in `axiolid-reference` and are tested there. What is pinned
//! here is the structure those consumers rely on.

use axiolid_core::{Frame3, Point3, Scalar, Vec3};
use axiolid_surface::{BSplineSurface, Cone, Cylinder, Plane, Sphere, Surface, Torus};

fn frame() -> Frame3 {
    Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

/// Every `Surface` variant must be constructible from public API alone.
#[test]
fn every_surface_variant_is_constructible() {
    let surfaces = [
        Surface::Plane(Plane { frame: frame() }),
        Surface::Cylinder(Cylinder {
            frame: frame(),
            radius: 1.0,
        }),
        Surface::Cone(Cone {
            frame: frame(),
            radius: 1.0,
            semi_angle: 0.5,
        }),
        Surface::Sphere(Sphere {
            frame: frame(),
            radius: 1.0,
        }),
        Surface::Torus(Torus {
            frame: frame(),
            major_radius: 3.0,
            minor_radius: 1.0,
        }),
        Surface::BSpline(BSplineSurface {
            u_degree: 1,
            v_degree: 1,
            control_points: vec![
                vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
                vec![Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
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
        }),
    ];
    assert_eq!(surfaces.len(), 6, "one instance per Surface variant");
}

/// `Surface` is `#[non_exhaustive]`, so consumers must keep a wildcard arm.
///
/// `axiolid-reference`'s `invert` and evaluators refuse unknown variants
/// explicitly instead of falling through. That is only sound while this enum
/// stays non-exhaustive.
#[test]
fn surface_enum_stays_non_exhaustive_for_consumers() {
    let surface = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 2.0,
    });
    let named = match surface {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cylinder",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        Surface::BSpline(_) => "bspline",
        // Required by non-exhaustiveness. See the curve counterpart.
        _ => "unknown",
    };
    assert_eq!(named, "sphere");
}

/// A surface's frame is stored, not normalised on construction.
///
/// Consumers that map between world and surface-local coordinates must
/// validate orthonormality themselves; `axiolid-reference`'s inversion does
/// exactly that. Storing a skewed frame is therefore representable, and this
/// pins that the vocabulary does not silently repair it.
#[test]
fn frames_are_stored_verbatim_not_normalised() {
    let skewed = Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X * 2.0,
        y: Vec3::Y,
        z: Vec3::Z,
    };
    let cylinder = Cylinder {
        frame: skewed,
        radius: 1.0,
    };
    assert_eq!(
        cylinder.frame.x,
        Vec3::X * 2.0,
        "a non-unit axis survives construction; validation is the consumer's job"
    );
}

/// The spline control net is a nested, strictly rectangular grid.
///
/// `control_points` is `Vec<Vec<Point3>>` rather than a flat vector with
/// declared extents, so "rectangular" is an invariant of the nesting: every
/// row must have equal length. A ragged net is representable in the type but
/// meaningless as a tensor-product surface, and readers index `[i][j]`
/// assuming it cannot happen. Weight nets, when present, must match the same
/// shape.
#[test]
fn spline_surface_control_net_is_a_rectangular_nested_grid() {
    let surface = BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![
            vec![Point3::ZERO, Point3::new(1.0, 0.0, 0.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![1.0, 0.5]]),
        u_closed: false,
        v_closed: false,
        knot_spec: axiolid_curve::KnotSpec::Unspecified,
        self_intersect: None,
    };

    let columns = surface.control_points[0].len();
    assert!(
        surface
            .control_points
            .iter()
            .all(|row| row.len() == columns),
        "every control-net row has equal length: the net is rectangular"
    );

    let weights = surface.weights.as_ref().expect("weights present");
    assert_eq!(
        weights.len(),
        surface.control_points.len(),
        "weight net has one row per control row"
    );
    assert!(
        weights
            .iter()
            .zip(&surface.control_points)
            .all(|(w, c)| w.len() == c.len()),
        "weight net matches the control net shape exactly"
    );

    for (knots, mults) in [
        (&surface.u_knots, &surface.u_multiplicities),
        (&surface.v_knots, &surface.v_multiplicities),
    ] {
        assert_eq!(
            knots.len(),
            mults.len(),
            "each parametric direction pairs knots with multiplicities"
        );
    }
}

/// A torus carries both radii independently, including the degenerate ordering.
///
/// `minor_radius >= major_radius` describes a self-intersecting spindle torus.
/// The vocabulary represents it; refusing or handling it is a consumer
/// decision. Pinning this keeps a well-formedness check from migrating here by
/// accident.
#[test]
fn torus_represents_the_spindle_case_without_refusing_it() {
    let spindle = Torus {
        frame: frame(),
        major_radius: 1.0,
        minor_radius: 2.0,
    };
    assert!(
        spindle.minor_radius > spindle.major_radius,
        "the spindle ordering is representable in the vocabulary"
    );
    let radii: Scalar = spindle.major_radius + spindle.minor_radius;
    assert_eq!(radii, 3.0);
}
