//! `invert` must be the exact inverse of `evaluate`.
//!
//! Each test drives the forward map, then requires the inverse to return
//! parameters that reproduce the same point. Round-tripping through the
//! real evaluator keeps the two definitions from drifting apart: a change
//! to either parameterisation breaks these without anyone remembering to
//! update a hand-written constant.

use axiolid_core::{Frame3, Point3, Tolerance, Vec3};
use axiolid_reference::surface::{evaluate, invert};
use axiolid_surface::{Cone, Cylinder, Plane, Sphere, Surface, Torus};

fn frame() -> Frame3 {
    Frame3 {
        origin: Point3::new(1.0, -2.0, 0.5),
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

fn round_trip(surface: &Surface, u: f64, v: f64) {
    let point = evaluate(surface, u, v).expect("forward");
    let (iu, iv) = invert(surface, point, Tolerance::MILLIMETRE).expect("inverse");
    let back = evaluate(surface, iu, iv).expect("forward again");
    let drift = (back - point).length();
    assert!(
        drift < 1e-9,
        "inverse named ({iu}, {iv}) which evaluates {drift} away from the original point"
    );
}

#[test]
fn a_plane_round_trips() {
    let s = Surface::Plane(Plane { frame: frame() });
    for (u, v) in [(0.0, 0.0), (3.5, -2.25), (-7.0, 11.0)] {
        round_trip(&s, u, v);
    }
}

#[test]
fn a_cylinder_round_trips() {
    let s = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: 2.5,
    });
    for (u, v) in [(0.0, 0.0), (1.0, 3.0), (-2.5, -4.0), (3.0, 0.25)] {
        round_trip(&s, u, v);
    }
}

#[test]
fn a_cone_round_trips() {
    let s = Surface::Cone(Cone {
        frame: frame(),
        radius: 3.0,
        semi_angle: 0.4,
    });
    for (u, v) in [(0.0, 0.0), (1.2, 2.0), (-2.0, -1.0)] {
        round_trip(&s, u, v);
    }
}

#[test]
fn a_sphere_round_trips() {
    let s = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 4.0,
    });
    // v stays off the poles, where u is genuinely unrecoverable.
    for (u, v) in [(0.0, 0.0), (1.0, 0.7), (-2.0, -1.2), (2.9, 0.1)] {
        round_trip(&s, u, v);
    }
}

#[test]
fn a_torus_round_trips() {
    let s = Surface::Torus(Torus {
        frame: frame(),
        major_radius: 6.0,
        minor_radius: 1.5,
    });
    for (u, v) in [(0.0, 0.0), (1.0, 2.0), (-2.0, -0.5), (3.0, 3.0)] {
        round_trip(&s, u, v);
    }
}

/// A point off the surface is refused, not projected onto it.
///
/// Projection would succeed silently and hand back parameters for a
/// DIFFERENT point, tilting every frame built from the result. The caller
/// asked which parameters name this point; if none do, that is the answer.
#[test]
fn a_point_off_the_surface_is_refused() {
    let s = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: 2.0,
    });
    let on = evaluate(&s, 0.5, 1.0).expect("forward");
    // Push it a centimetre off the surface, well beyond millimetre tolerance.
    let normal_dir = (on - Point3::new(1.0, -2.0, 1.5)).normalize();
    let off = on + normal_dir * 0.01;
    assert!(
        invert(&s, off, Tolerance::MILLIMETRE).is_err(),
        "a point off the surface must not be silently projected onto it"
    );
    // Control: the unmoved point still inverts, so the refusal above is
    // attributable to the offset and not to the fixture.
    assert!(invert(&s, on, Tolerance::MILLIMETRE).is_ok());
}

/// At a sphere pole every u names the same point, so u is refused.
#[test]
fn a_sphere_pole_is_refused() {
    let s = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 3.0,
    });
    let pole = evaluate(&s, 0.0, core::f64::consts::FRAC_PI_2).expect("forward");
    assert!(
        invert(&s, pole, Tolerance::MILLIMETRE).is_err(),
        "the pole has no unique angular parameter and must be refused"
    );
}

/// At a cone apex the radius vanishes and u is likewise unrecoverable.
#[test]
fn a_cone_apex_is_refused() {
    let radius = 2.0;
    let semi_angle = 0.5_f64;
    let s = Surface::Cone(Cone {
        frame: frame(),
        radius,
        semi_angle,
    });
    // The apex sits where radius + v*tan(semi_angle) == 0.
    let v_apex = -radius / semi_angle.tan();
    let apex = evaluate(&s, 0.0, v_apex).expect("forward");
    assert!(
        invert(&s, apex, Tolerance::MILLIMETRE).is_err(),
        "the apex has no unique angular parameter and must be refused"
    );
}

/// A B-spline has no closed form, and the gap is named rather than
/// approximated by an analytic branch that does not describe it.
#[test]
fn a_bspline_is_refused_by_name() {
    use axiolid_surface::BSplineSurface;
    let s = Surface::BSpline(BSplineSurface {
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
    });
    let err = invert(&s, Point3::ZERO, Tolerance::MILLIMETRE).expect_err("must refuse");
    assert!(
        matches!(err, axiolid_contracts::GeomError::Unsupported { .. }),
        "expected a named capability refusal, got {err:?}"
    );
}

/// A non-orthonormal frame is refused before it can mislead.
///
/// The dot-product inverse is only valid on an orthonormal frame. On a
/// skewed or scaled one it returns coordinates that are wrong in a way
/// that still ROUND-TRIPS: `evaluate` uses the same bad axes, so the
/// residual check cannot see the error. Each fixture below is therefore
/// chosen so that the residual check would pass, leaving the frame
/// validation as the only thing standing between the caller and a
/// silently mis-parameterised sweep.
#[test]
fn a_skewed_frame_is_refused() {
    // A scaled x axis: place() stretches local x by 2, and to_local's
    // projection divides by 1, so the pair are inconsistent.
    let scaled = Surface::Plane(Plane {
        frame: Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X * 2.0,
            y: Vec3::Y,
            z: Vec3::Z,
        },
    });
    // Probed ON the good axis: place(to_local(p)) == p here because the
    // point avoids the scaled axis entirely, so the residual check sees a
    // perfect round trip and waves it through. Only the frame validation
    // can catch this, which is exactly why it is not redundant.
    assert!(
        invert(&scaled, Point3::new(0.0, 1.0, 0.0), Tolerance::MILLIMETRE).is_err(),
        "a scaled frame must be refused even where the round trip closes"
    );

    let shear = Vec3::new(1.0, 1.0, 0.0).normalize();
    // A sheared CYLINDER is the case the residual check cannot see. Its
    // axes are all unit length, so only perpendicularity is violated, and
    // this point round trips to within 8.6e-4 -- inside millimetre
    // tolerance -- while naming an angle 8.0e-4 rad away from the truth.
    // Without the perpendicularity guard the caller receives confidently
    // wrong parameters and every section frame built from them leans.
    let sheared_cylinder = Surface::Cylinder(Cylinder {
        frame: Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X,
            y: shear,
            z: Vec3::Z,
        },
        radius: 2.0,
    });
    assert!(
        invert(
            &sheared_cylinder,
            Point3::new(2.4138136269623236, 1.0009623643877532, -2.2296521298319587),
            Tolerance::MILLIMETRE,
        )
        .is_err(),
        "non-perpendicular axes must be refused even where the residual \
         check is satisfied"
    );

    // Control: the same query on a proper frame succeeds, so the refusals
    // above are attributable to the frames and not to the points.
    let clean = Surface::Plane(Plane {
        frame: Frame3 {
            origin: Point3::ZERO,
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        },
    });
    assert!(invert(&clean, Point3::new(1.0, 0.0, 0.0), Tolerance::MILLIMETRE).is_ok());
}
