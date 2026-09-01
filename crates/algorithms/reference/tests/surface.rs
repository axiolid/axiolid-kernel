//! Surface evaluation against closed-form truth (ADR 0012).
//!
//! Every constant here is derivable on paper. Where a value is integrated
//! numerically the integrand is the analytic surface itself, so the test
//! cannot agree with the implementation by sharing its mistake.

use axiolid_core::{Frame3, Point3, Scalar, Vec3};
use axiolid_curve::KnotSpec;
use axiolid_reference::surface::{evaluate, normal, partials, Patch};
use axiolid_surface::{BSplineSurface, Cone, Cylinder, Plane, Sphere, Surface, Torus};

const TAU: Scalar = core::f64::consts::TAU;
const PI: Scalar = core::f64::consts::PI;

fn frame() -> Frame3 {
    Frame3 {
        origin: Point3::new(0.0, 0.0, 0.0),
        x: Vec3::new(1.0, 0.0, 0.0),
        y: Vec3::new(0.0, 1.0, 0.0),
        z: Vec3::new(0.0, 0.0, 1.0),
    }
}

/// A deliberately rotated, translated frame: every family must respect its
/// own frame rather than assuming world axes.
fn tilted() -> Frame3 {
    let inv = 1.0 / (2.0 as Scalar).sqrt();
    Frame3 {
        origin: Point3::new(3.0, -2.0, 7.0),
        x: Vec3::new(inv, inv, 0.0),
        y: Vec3::new(-inv, inv, 0.0),
        z: Vec3::new(0.0, 0.0, 1.0),
    }
}

/// Surface area by the first fundamental form, integrated with a periodic
/// trapezoid rule (spectrally accurate for a closed smooth surface).
///
/// Uses central differences of `evaluate`, so it tests the evaluated surface,
/// not the normal implementation.
fn area(surface: &Surface, patch: Patch, n: usize) -> Scalar {
    let hu = (patch.u_end - patch.u_start) / n as Scalar;
    let hv = (patch.v_end - patch.v_start) / n as Scalar;
    let eps = 1e-7;
    let mut total = 0.0;
    for i in 0..n {
        for j in 0..n {
            let u = patch.u_start + (i as Scalar + 0.5) * hu;
            let v = patch.v_start + (j as Scalar + 0.5) * hv;
            let du = (evaluate(surface, u + eps, v).unwrap()
                - evaluate(surface, u - eps, v).unwrap())
                / (2.0 * eps);
            let dv = (evaluate(surface, u, v + eps).unwrap()
                - evaluate(surface, u, v - eps).unwrap())
                / (2.0 * eps);
            total += du.cross(dv).length() * hu * hv;
        }
    }
    total
}

// --- plane ------------------------------------------------------------------

#[test]
fn a_plane_is_exact_and_flat() {
    let p = Surface::Plane(Plane { frame: tilted() });
    let f = tilted();
    for (u, v) in [(0.0, 0.0), (2.0, -3.0), (-1.5, 4.25)] {
        let got = evaluate(&p, u, v).expect("plane evaluates");
        let want = f.origin + f.x * u + f.y * v;
        assert!((got - want).length() < 1e-12, "plane at ({u},{v}): {got:?}");
    }
    // The normal is the frame z everywhere, including far from the origin.
    let n = normal(&p, 100.0, -100.0).expect("plane normal");
    assert!((n - f.z).length() < 1e-12, "plane normal {n:?}");
}

// --- cylinder ---------------------------------------------------------------

#[test]
fn a_cylinder_stays_on_its_radius_and_axis() {
    let r = 2.5;
    let c = Surface::Cylinder(Cylinder {
        frame: tilted(),
        radius: r,
    });
    let f = tilted();
    for k in 0..16 {
        let u = TAU * k as Scalar / 16.0;
        let v = 1.75;
        let p = evaluate(&c, u, v).expect("cylinder evaluates");
        // Distance to the axis is exactly the radius.
        let rel = p - (f.origin + f.z * v);
        assert!(
            (rel.length() - r).abs() < 1e-12,
            "cylinder radius at u={u}: {}",
            rel.length()
        );
        // The normal is radial: perpendicular to the axis, unit length.
        let n = normal(&c, u, v).expect("cylinder normal");
        assert!(
            n.dot(f.z).abs() < 1e-12,
            "normal must be perpendicular to axis"
        );
        assert!((n.length() - 1.0).abs() < 1e-12);
        // And it points away from the axis.
        assert!(n.dot(rel) > 0.0, "cylinder normal must point outward");
    }
}

#[test]
fn a_cylinder_patch_has_area_tau_r_h() {
    let (r, h) = (1.5, 4.0);
    let c = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: r,
    });
    let patch = Patch::full_turn(0.0, h).expect("patch");
    let got = area(&c, patch, 64);
    let want = TAU * r * h;
    assert!(
        (got - want).abs() / want < 1e-9,
        "cylinder area {got} vs {want}"
    );
}

// --- sphere -----------------------------------------------------------------

#[test]
fn a_sphere_stays_on_its_radius() {
    let r = 3.0;
    let s = Surface::Sphere(Sphere {
        frame: tilted(),
        radius: r,
    });
    let origin = tilted().origin;
    for k in 0..12 {
        for l in 0..7 {
            let u = TAU * k as Scalar / 12.0;
            let v = -PI / 2.0 + PI * l as Scalar / 6.0;
            let p = evaluate(&s, u, v).expect("sphere evaluates");
            assert!(
                ((p - origin).length() - r).abs() < 1e-12,
                "sphere radius at ({u},{v}): {}",
                (p - origin).length()
            );
        }
    }
}

#[test]
fn a_sphere_has_area_four_pi_r_squared() {
    let r = 2.0;
    let s = Surface::Sphere(Sphere {
        frame: frame(),
        radius: r,
    });
    // Poles excluded by a hair: the parameterisation is singular there, and
    // the integrand is what is being tested, not the pole handling.
    let patch = Patch::new(0.0, TAU, -PI / 2.0, PI / 2.0).expect("patch");
    let got = area(&s, patch, 128);
    let want = 4.0 * PI * r * r;
    // The midpoint rule here is second order, not spectral: the integrand is
    // not smooth-periodic in v across the poles. Measured convergence is
    // 1.00e-4 (n=64), 2.51e-5 (n=128), 6.28e-6 (n=256) -- a clean 4x per
    // doubling. Asserting 1e-6 at n=128 would assert a rate the rule lacks,
    // so the bound matches the quadrature and the RATE is asserted below.
    assert!(
        (got - want).abs() / want < 1e-4,
        "sphere area {got} vs {want}"
    );
    // The rate is the real claim: halving h must quarter the error. A wrong
    // surface would not converge at second order toward the analytic value.
    let coarse = (area(&s, patch, 64) - want).abs() / want;
    let fine = (area(&s, patch, 128) - want).abs() / want;
    assert!(
        fine < coarse / 3.0,
        "sphere area must converge at second order: {coarse} -> {fine}"
    );
}

#[test]
fn a_sphere_normal_is_the_outward_radial_direction() {
    let s = Surface::Sphere(Sphere {
        frame: tilted(),
        radius: 1.25,
    });
    let origin = tilted().origin;
    for (u, v) in [(0.3, 0.4), (2.0, -0.9), (5.5, 1.2)] {
        let p = evaluate(&s, u, v).expect("evaluate");
        let n = normal(&s, u, v).expect("normal");
        let radial = (p - origin).normalize();
        assert!(
            (n - radial).length() < 1e-12,
            "sphere normal {n:?} vs radial {radial:?}"
        );
    }
}

// --- torus ------------------------------------------------------------------

#[test]
fn a_torus_has_area_four_pi_squared_r_r() {
    let (major, minor) = (3.0, 1.0);
    let t = Surface::Torus(Torus {
        frame: frame(),
        major_radius: major,
        minor_radius: minor,
    });
    let patch = Patch::new(0.0, TAU, 0.0, TAU).expect("patch");
    let got = area(&t, patch, 128);
    // Pappus: A = (2*pi*R)(2*pi*r) = 4*pi^2*R*r.
    let want = 4.0 * PI * PI * major * minor;
    // Both parameters are periodic and the integrand is smooth, so the
    // midpoint rule is spectrally accurate: an independent evaluation of the
    // same integrand agrees to 2.9e-15 at n=64. The residual here is the
    // 1e-7 central-difference step used for the tangents, whose error is
    // O(h^2) = 1e-14 relative per sample but accumulates through the cross
    // product; 1e-8 is the honest floor for a differenced tangent.
    assert!(
        (got - want).abs() / want < 1e-8,
        "torus area {got} vs {want}"
    );
}

#[test]
fn a_torus_tube_stays_at_the_minor_radius() {
    let (major, minor) = (5.0, 1.5);
    let t = Surface::Torus(Torus {
        frame: frame(),
        major_radius: major,
        minor_radius: minor,
    });
    for k in 0..8 {
        for l in 0..8 {
            let u = TAU * k as Scalar / 8.0;
            let v = TAU * l as Scalar / 8.0;
            let p = evaluate(&t, u, v).expect("torus evaluates");
            // Centre of the tube at this u.
            let (s, c) = u.sin_cos();
            let centre = Point3::new(major * c, major * s, 0.0);
            assert!(
                ((p - centre).length() - minor).abs() < 1e-12,
                "torus tube radius at ({u},{v}): {}",
                (p - centre).length()
            );
        }
    }
}

// --- cone -------------------------------------------------------------------

#[test]
fn a_cone_radius_shrinks_at_the_semi_angle() {
    let c = Surface::Cone(Cone {
        frame: frame(),
        radius: 2.0,
        // 45 degrees: radius falls by exactly 1 per unit height.
        semi_angle: -PI / 4.0,
    });
    for v in [0.0, 0.5, 1.0, 1.5] {
        let p = evaluate(&c, 0.0, v).expect("cone evaluates");
        let want = 2.0 - v;
        assert!(
            (p.x - want).abs() < 1e-12,
            "cone radius at v={v}: {} want {want}",
            p.x
        );
    }
}

#[test]
fn a_cone_patch_crossing_the_apex_is_refused() {
    let c = Surface::Cone(Cone {
        frame: frame(),
        radius: 1.0,
        semi_angle: -PI / 4.0,
    });
    // At v = 2 the radius would be -1: past the apex, not a valid surface.
    assert!(
        evaluate(&c, 0.0, 2.0).is_err(),
        "a cone past its apex must be refused, not folded"
    );
}

#[test]
fn a_cone_normal_leans_by_its_semi_angle() {
    let semi = PI / 6.0;
    let c = Surface::Cone(Cone {
        frame: frame(),
        radius: 1.0,
        semi_angle: semi,
    });
    let n = normal(&c, 0.0, 0.0).expect("cone normal");
    // At u=0 the outward radial direction is +x; the normal tilts by the
    // semi-angle toward -z.
    assert!((n.x - semi.cos()).abs() < 1e-12, "normal x {}", n.x);
    assert!((n.z + semi.sin()).abs() < 1e-12, "normal z {}", n.z);
    assert!((n.length() - 1.0).abs() < 1e-12);
}

// --- B-spline surface -------------------------------------------------------

/// A bilinear clamped B-spline over a 2x2 net is exactly a bilinear patch.
fn bilinear(corners: [[Point3; 2]; 2]) -> BSplineSurface {
    BSplineSurface {
        u_degree: 1,
        v_degree: 1,
        control_points: vec![corners[0].to_vec(), corners[1].to_vec()],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![2, 2],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: None,
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::PiecewiseBezier,
        self_intersect: None,
    }
}

#[test]
fn a_bilinear_bspline_matches_the_closed_form() {
    let net = [
        [Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 1.0)],
        [Point3::new(3.0, 0.0, 2.0), Point3::new(3.0, 2.0, 5.0)],
    ];
    let s = Surface::BSpline(bilinear(net));
    for (u, v) in [(0.0, 0.0), (1.0, 1.0), (0.25, 0.75), (0.5, 0.5)] {
        let got = evaluate(&s, u, v).expect("bspline evaluates");
        // Bilinear interpolation of the four corners.
        let want = net[0][0] * ((1.0 - u) * (1.0 - v))
            + net[0][1] * ((1.0 - u) * v)
            + net[1][0] * (u * (1.0 - v))
            + net[1][1] * (u * v);
        assert!(
            (got - want).length() < 1e-12,
            "bilinear at ({u},{v}): {got:?} vs {want:?}"
        );
    }
}

#[test]
fn a_bspline_surface_interpolates_its_corner_control_points() {
    let net = [
        [Point3::new(-1.0, -1.0, 0.0), Point3::new(-1.0, 1.0, 0.0)],
        [Point3::new(1.0, -1.0, 0.0), Point3::new(1.0, 1.0, 4.0)],
    ];
    let s = Surface::BSpline(bilinear(net));
    let corners = [
        ((0.0, 0.0), net[0][0]),
        ((0.0, 1.0), net[0][1]),
        ((1.0, 0.0), net[1][0]),
        ((1.0, 1.0), net[1][1]),
    ];
    for ((u, v), want) in corners {
        let got = evaluate(&s, u, v).expect("evaluate");
        assert!(
            (got - want).length() < 1e-12,
            "clamped corner ({u},{v}): {got:?} vs {want:?}"
        );
    }
}

/// A rational B-spline surface reproduces a quarter cylinder exactly.
///
/// The classic NURBS circle weights (1, 1/sqrt2, 1) along `u`, extruded
/// linearly along `v`. This is the test that distinguishes real homogeneous
/// interpolation from projecting first and averaging.
#[test]
fn a_rational_bspline_surface_is_exactly_a_quarter_cylinder() {
    let w = 1.0 / (2.0 as Scalar).sqrt();
    let s = Surface::BSpline(BSplineSurface {
        u_degree: 2,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 2.0)],
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 2.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 2.0)],
        ],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![3, 3],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![w, w], vec![1.0, 1.0]]),
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::PiecewiseBezier,
        self_intersect: None,
    });
    for k in 0..=10 {
        let u = k as Scalar / 10.0;
        for v in [0.0, 0.5, 1.0] {
            let p = evaluate(&s, u, v).expect("evaluate");
            let radius = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                (radius - 1.0).abs() < 1e-12,
                "quarter cylinder radius at ({u},{v}): {radius}"
            );
            // And the height is the linear v direction.
            assert!((p.z - 2.0 * v).abs() < 1e-12, "height {}", p.z);
        }
    }
}

#[test]
fn rational_bspline_partials_match_the_quarter_cylinder_hodographs() {
    let w = 1.0 / (2.0 as Scalar).sqrt();
    let surface = Surface::BSpline(BSplineSurface {
        u_degree: 2,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 2.0)],
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 2.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 2.0)],
        ],
        u_knots: vec![0.0, 1.0],
        u_multiplicities: vec![3, 3],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![w, w], vec![1.0, 1.0]]),
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::PiecewiseBezier,
        self_intersect: None,
    });

    let (du, dv) = partials(&surface, 0.5, 0.25).expect("analytic partials");
    let tangent = 2.0 / (1.0 + w);
    let expected_du = Vec3::new(-tangent, tangent, 0.0);
    let expected_dv = Vec3::new(0.0, 0.0, 2.0);
    assert!(
        (du - expected_du).length() < 1e-12,
        "du {du:?} vs {expected_du:?}"
    );
    assert!(
        (dv - expected_dv).length() < 1e-12,
        "dv {dv:?} vs {expected_dv:?}"
    );

    // Off symmetry the homogeneous weight derivative is non-zero. This pins
    // the quotient-rule correction rather than only the polynomial hodograph.
    let u = 0.25;
    let b0 = (1.0 - u) * (1.0 - u);
    let b1 = 2.0 * u * (1.0 - u);
    let b2 = u * u;
    let db0 = -2.0 * (1.0 - u);
    let db1 = 2.0 - 4.0 * u;
    let db2 = 2.0 * u;
    let weight = b0 + b1 * w + b2;
    let dweight = db0 + db1 * w + db2;
    let point_h = Vec3::new(b0 + b1 * w, b1 * w + b2, 0.5 * weight);
    let derivative_h = Vec3::new(db0 + db1 * w, db1 * w + db2, 0.5 * dweight);
    let expected = (derivative_h - point_h * (dweight / weight)) / weight;
    let (du, _) = partials(&surface, u, 0.25).expect("off-symmetry analytic partials");
    assert!(
        (du - expected).length() < 1e-12,
        "off-symmetry du {du:?} vs {expected:?}"
    );
}

#[test]
fn a_bspline_normal_survives_a_large_parameter_offset() {
    // CAD exchange files often retain large native knot origins. At 1e12 the
    // old `(domain width) * 1e-6` finite-difference step rounds to zero, even
    // though this is the same regular quarter cylinder as the unit-domain case.
    let w = 1.0 / (2.0 as Scalar).sqrt();
    let start = 1.0e12;
    let end = start + 1.0e-3;
    let surface = Surface::BSpline(BSplineSurface {
        u_degree: 2,
        v_degree: 1,
        control_points: vec![
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 2.0)],
            vec![Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 2.0)],
            vec![Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 1.0, 2.0)],
        ],
        u_knots: vec![start, end],
        u_multiplicities: vec![3, 3],
        v_knots: vec![0.0, 1.0],
        v_multiplicities: vec![2, 2],
        weights: Some(vec![vec![1.0, 1.0], vec![w, w], vec![1.0, 1.0]]),
        u_closed: false,
        v_closed: false,
        knot_spec: KnotSpec::PiecewiseBezier,
        self_intersect: None,
    });

    let u = start + (end - start) * 0.5;
    let n = normal(&surface, u, 0.5).expect("regular normal at large knot origin");
    let expected = Vec3::new(1.0, 1.0, 0.0).normalize();
    assert!(
        (n - expected).length() < 1e-12,
        "normal {n:?} vs {expected:?}"
    );
}

#[test]
fn a_bspline_normal_is_perpendicular_to_the_surface() {
    let net = [
        [Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ];
    // A flat unit square in z = 0: the normal must be +/- z everywhere.
    let s = Surface::BSpline(bilinear(net));
    let n = normal(&s, 0.5, 0.5).expect("normal");
    assert!(
        (n.x.abs() < 1e-9) && (n.y.abs() < 1e-9) && ((n.z.abs() - 1.0).abs() < 1e-9),
        "flat patch normal must be axial, got {n:?}"
    );
}

// --- refusals ---------------------------------------------------------------

#[test]
fn mismatched_surface_knots_are_refused_instead_of_truncated() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.u_knots.push(2.0);
    assert!(evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err());
}

#[test]
fn non_monotone_surface_knots_are_refused() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.u_knots = vec![0.0, 2.0, 1.0];
    b.u_multiplicities = vec![2, 1, 1];
    b.control_points
        .push(vec![Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 1.0, 0.0)]);
    assert!(evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err());
}

#[test]
fn non_positive_surface_weights_are_refused() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.weights = Some(vec![vec![1.0, 0.0], vec![1.0, 1.0]]);
    assert!(evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err());
}

#[test]
fn non_finite_surface_control_points_are_refused() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.control_points[1][1].z = Scalar::INFINITY;
    assert!(evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err());
}

#[test]
fn a_ragged_control_net_is_refused() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.control_points[1].push(Point3::new(2.0, 2.0, 2.0));
    assert!(
        evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err(),
        "a ragged net evaluates a different surface than declared: refuse it"
    );
}

#[test]
fn a_mismatched_knot_vector_is_refused() {
    let mut b = bilinear([
        [Point3::ZERO, Point3::new(0.0, 1.0, 0.0)],
        [Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
    ]);
    b.u_multiplicities = vec![2, 3];
    assert!(evaluate(&Surface::BSpline(b), 0.5, 0.5).is_err());
}

#[test]
fn a_non_positive_radius_is_refused() {
    let s = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 0.0,
    });
    assert!(evaluate(&s, 0.0, 0.0).is_err());
    let c = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: -1.0,
    });
    assert!(evaluate(&c, 0.0, 0.0).is_err());
}

#[test]
fn a_non_finite_parameter_is_refused() {
    let s = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 1.0,
    });
    assert!(evaluate(&s, Scalar::NAN, 0.0).is_err());
    assert!(evaluate(&s, 0.0, Scalar::INFINITY).is_err());
    assert!(normal(&s, Scalar::NAN, 0.0).is_err());
}

#[test]
fn an_empty_patch_is_refused() {
    assert!(Patch::new(0.0, 0.0, 0.0, 1.0).is_err(), "zero u extent");
    assert!(Patch::new(0.0, 1.0, 2.0, 1.0).is_err(), "reversed v");
    assert!(
        Patch::new(Scalar::NAN, 1.0, 0.0, 1.0).is_err(),
        "non-finite"
    );
}

#[test]
fn rational_surface_overflow_is_refused() {
    let huge = Point3::splat(Scalar::MAX);
    let mut spline = bilinear([[huge; 2]; 2]);
    spline.weights = Some(vec![vec![Scalar::MAX; 2]; 2]);
    let surface = Surface::BSpline(spline);
    assert!(evaluate(&surface, 0.5, 0.5).is_err());
    assert!(partials(&surface, 0.5, 0.5).is_err());
}

#[test]
fn non_finite_surface_frames_are_refused() {
    let mut invalid = frame();
    invalid.origin.x = Scalar::INFINITY;
    let surface = Surface::Plane(Plane { frame: invalid });
    assert!(evaluate(&surface, 0.0, 0.0).is_err());
    assert!(partials(&surface, 0.0, 0.0).is_err());
    assert!(normal(&surface, 0.0, 0.0).is_err());
}
