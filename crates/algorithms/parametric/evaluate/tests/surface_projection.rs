//! Projection names the NEAREST point; inversion names an on-surface one.

use axiolid_core::{Frame3, Point3, Scalar, Tolerance, Vec3};
use axiolid_evaluate::surface::{evaluate, invert, project};
use axiolid_surface::{Cone, Cylinder, Sphere, Surface, Torus};

fn tol() -> Tolerance {
    Tolerance::new(1e-9, 1e-12).expect("valid tolerance")
}

fn frame() -> Frame3 {
    Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    }
}

/// Brute-force nearest point, the independent oracle.
fn brute_force(
    surface: &Surface,
    point: Point3,
    u_range: (Scalar, Scalar),
    v_range: (Scalar, Scalar),
) -> Scalar {
    let mut best = Scalar::INFINITY;
    let steps = 900;
    for i in 0..=steps {
        let u = u_range.0 + (u_range.1 - u_range.0) * (i as Scalar) / (steps as Scalar);
        for j in 0..=steps {
            let v = v_range.0 + (v_range.1 - v_range.0) * (j as Scalar) / (steps as Scalar);
            if let Ok(q) = evaluate(surface, u, v) {
                best = best.min((q - point).length());
            }
        }
    }
    best
}

fn projected_distance(surface: &Surface, point: Point3) -> Scalar {
    let (u, v) = project(surface, point, tol()).expect("projects");
    let landed = evaluate(surface, u, v).expect("evaluates");
    (landed - point).length()
}

/// The whole reason projection exists: inversion refuses an off-surface point.
#[test]
fn inversion_refuses_what_projection_answers() {
    let cylinder = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: 1.0,
    });
    // The midpoint of a chord across a faceted cylinder: strictly inside.
    let inside = Point3::new(0.853_553_390_6, 0.353_553_390_6, 0.0);

    assert!(
        invert(&cylinder, inside, tol()).is_err(),
        "inversion must refuse a point that is not on the surface"
    );
    let (u, v) = project(&cylinder, inside, tol()).expect("projection answers it");
    let landed = evaluate(&cylinder, u, v).expect("evaluates");
    let radius = (landed.x * landed.x + landed.y * landed.y).sqrt();
    assert!(
        (radius - 1.0).abs() < 1e-12,
        "projected point must lie ON the cylinder, got radius {radius}"
    );
}

#[test]
fn cylinder_projection_is_the_true_nearest_point() {
    let surface = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: 1.0,
    });
    let point = Point3::new(0.4, 0.3, 0.7);
    let ours = projected_distance(&surface, point);
    let oracle = brute_force(&surface, point, (0.0, std::f64::consts::TAU), (0.7, 0.7));
    assert!(
        (ours - oracle).abs() < 1e-6,
        "cylinder: ours {ours} vs brute force {oracle}"
    );
}

#[test]
fn sphere_projection_is_the_true_nearest_point() {
    let surface = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 2.0,
    });
    let point = Point3::new(0.9, -0.4, 1.1);
    let ours = projected_distance(&surface, point);
    let half = std::f64::consts::FRAC_PI_2;
    let oracle = brute_force(&surface, point, (0.0, std::f64::consts::TAU), (-half, half));
    assert!(
        (ours - oracle).abs() < 1e-5,
        "sphere: ours {ours} vs brute force {oracle}"
    );
}

/// A cone's nearest point is along the slant, not at constant height.
///
/// A radial-only projection passes a cylinder test but fails here, so this
/// is what separates a correct cone arm from a copied one.
#[test]
fn cone_projection_follows_the_slant_not_the_radius() {
    let surface = Surface::Cone(Cone {
        frame: frame(),
        radius: 1.0,
        semi_angle: std::f64::consts::FRAC_PI_6,
    });
    let point = Point3::new(2.0, 0.0, 0.5);
    let ours = projected_distance(&surface, point);
    let oracle = brute_force(&surface, point, (0.0, std::f64::consts::TAU), (-2.0, 4.0));
    assert!(
        (ours - oracle).abs() < 1e-5,
        "cone: ours {ours} vs brute force {oracle}"
    );

    let (_, v) = project(&surface, point, tol()).expect("projects");
    assert!(
        (v - 0.5).abs() > 1e-6,
        "the foot must move along the slant, so v must differ from the point's own z"
    );
}

#[test]
fn torus_projection_is_the_true_nearest_point() {
    let surface = Surface::Torus(Torus {
        frame: frame(),
        major_radius: 3.0,
        minor_radius: 1.0,
    });
    let point = Point3::new(2.0, 0.0, 0.4);
    let ours = projected_distance(&surface, point);
    let oracle = brute_force(
        &surface,
        point,
        (0.0, std::f64::consts::TAU),
        (0.0, std::f64::consts::TAU),
    );
    // A sampled oracle cannot beat a closed form; it can only match it or
    // lose by its own grid resolution. Requiring `ours <= oracle` is the
    // honest assertion -- a symmetric tolerance would be testing the grid.
    assert!(
        ours <= oracle + 1e-9,
        "torus: closed form {ours} must not be worse than sampled {oracle}"
    );
}

/// An equidistant point has no nearest point, so naming one would be a lie.
#[test]
fn an_ambiguous_point_is_refused_rather_than_broken_arbitrarily() {
    let cylinder = Surface::Cylinder(Cylinder {
        frame: frame(),
        radius: 1.0,
    });
    let on_axis = Point3::new(0.0, 0.0, 0.5);
    let error = project(&cylinder, on_axis, tol()).expect_err("the axis is equidistant");
    assert!(
        format!("{error}").contains("not unique"),
        "the refusal must name the tie, got: {error}"
    );

    let sphere = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 1.0,
    });
    assert!(
        project(&sphere, Point3::ZERO, tol()).is_err(),
        "the sphere centre is equidistant from the whole surface"
    );
}

/// A point already on the surface must project to itself.
#[test]
fn projection_is_idempotent_on_the_surface() {
    let surface = Surface::Sphere(Sphere {
        frame: frame(),
        radius: 2.0,
    });
    let on_surface = evaluate(&surface, 0.7, 0.3).expect("evaluates");
    let distance = projected_distance(&surface, on_surface);
    assert!(
        distance < 1e-12,
        "a point on the surface must project onto itself, moved {distance}"
    );
}
