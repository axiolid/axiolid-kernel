//! Scalar reference implementation of surface evaluation (ADR 0012).
//!
//! # What this closes
//!
//! `axiolid-surface` declared six surface families and a `SurfaceEvaluator`
//! trait. Nothing implemented it, so a B-rep face on any curved surface could
//! not be tessellated, which is most faces in a real building model. This
//! reference that makes the declaration executable.
//!
//! # Parameterisation
//!
//! Each family uses the conventional parameterisation, chosen so `u` is the
//! angular direction wherever one exists (matching the curve module, where a
//! full turn is `[0, tau]`):
//!
//! | family   | `u`                  | `v`                     |
//! |----------|----------------------|-------------------------|
//! | Plane    | local x offset       | local y offset          |
//! | Cylinder | angle about z        | height along z          |
//! | Cone     | angle about z        | height along z          |
//! | Sphere   | azimuth about z      | polar, `-pi/2 .. pi/2`  |
//! | Torus    | angle about z        | angle around the tube   |
//! | BSpline  | first knot axis      | second knot axis        |
//!
//! Normals point outward for closed families (away from the axis for a
//! cylinder, away from the centre for a sphere, away from the tube centre for
//! a torus). A caller that needs the opposite convention negates; the kernel
//! does not guess.
//!
//! # What it does not do
//!
//! No surface-surface intersection, no trimming, no blending. Those are the
//! parts of a NURBS kernel this crate deliberately does not attempt: for a
//! tessellate-and-check pipeline the useful operation is evaluation, and the
//! boolean stack works on meshes.

use axiolid_core::{Frame3, Point3, Scalar, Vec3};
use axiolid_kernel::BackendId;
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_surface::{BSplineSurface, Cone, Cylinder, Plane, Sphere, Surface, Torus};

use crate::curve::{de_boor_recurrence, eval_homogeneous, span_in};
use crate::nurbs::SplineAxis;

/// A finite parameter rectangle for tessellation.
///
/// Elementary surfaces are infinite (a plane, a cylinder) or only periodic in
/// one direction, so a caller must say which patch it wants. Returning a
/// default would invent geometry the source never declared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// Start of the `u` interval.
    pub u_start: Scalar,
    /// End of the `u` interval.
    pub u_end: Scalar,
    /// Start of the `v` interval.
    pub v_start: Scalar,
    /// End of the `v` interval.
    pub v_end: Scalar,
}

impl Patch {
    /// Construct a patch, rejecting an empty or non-finite rectangle.
    pub fn new(u_start: Scalar, u_end: Scalar, v_start: Scalar, v_end: Scalar) -> GeomResult<Self> {
        let all = [u_start, u_end, v_start, v_end];
        if !all.iter().all(|value| value.is_finite()) {
            return Err(GeomError::InvalidInput(format!(
                "patch bounds must be finite, got {all:?}"
            )));
        }
        if !(u_end > u_start && v_end > v_start) {
            return Err(GeomError::Degenerate(format!(
                "patch must have positive extent, got u {u_start}..{u_end}, v {v_start}..{v_end}"
            )));
        }
        Ok(Self {
            u_start,
            u_end,
            v_start,
            v_end,
        })
    }

    /// The full closed patch for a family that is periodic in `u`.
    pub fn full_turn(v_start: Scalar, v_end: Scalar) -> GeomResult<Self> {
        Self::new(0.0, core::f64::consts::TAU, v_start, v_end)
    }
}

/// Map a local-frame point into world coordinates.
fn place(frame: &Frame3, local: Vec3) -> Point3 {
    frame.origin + frame.x * local.x + frame.y * local.y + frame.z * local.z
}

/// Map a local-frame direction into world coordinates (no translation).
fn direct(frame: &Frame3, local: Vec3) -> Vec3 {
    frame.x * local.x + frame.y * local.y + frame.z * local.z
}

fn finite(value: Scalar, what: &str) -> GeomResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(GeomError::InvalidInput(format!(
            "{what} must be finite, got {value}"
        )))
    }
}

fn positive(value: Scalar, what: &str) -> GeomResult<()> {
    finite(value, what)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(GeomError::InvalidInput(format!(
            "{what} must be positive, got {value}"
        )))
    }
}

fn finite_surface_frame(surface: &Surface) -> GeomResult<()> {
    let frame = match surface {
        Surface::Plane(value) => Some(&value.frame),
        Surface::Cylinder(value) => Some(&value.frame),
        Surface::Cone(value) => Some(&value.frame),
        Surface::Sphere(value) => Some(&value.frame),
        Surface::Torus(value) => Some(&value.frame),
        _ => None,
    };
    if frame.is_none_or(|frame| {
        frame.origin.is_finite()
            && frame.x.is_finite()
            && frame.y.is_finite()
            && frame.z.is_finite()
    }) {
        Ok(())
    } else {
        Err(GeomError::InvalidInput(
            "surface frame must be finite".to_owned(),
        ))
    }
}

/// Position on a surface at `(u, v)`.
pub fn evaluate(surface: &Surface, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    finite(u, "surface parameter u")?;
    finite(v, "surface parameter v")?;
    finite_surface_frame(surface)?;
    let point = match surface {
        Surface::Plane(p) => Ok(plane_point(p, u, v)),
        Surface::Cylinder(c) => cylinder_point(c, u, v),
        Surface::Cone(c) => cone_point(c, u, v),
        Surface::Sphere(s) => sphere_point(s, u, v),
        Surface::Torus(t) => torus_point(t, u, v),
        Surface::BSpline(b) => bspline_point(b, u, v),
        _ => Err(GeomError::Unsupported {
            backend: ScalarSurface::ID,
            operation: axiolid_kernel::Operation::SurfaceEvaluation,
        }),
    }?;
    if point.is_finite() {
        Ok(point)
    } else {
        Err(GeomError::Degenerate(
            "surface point is non-finite".to_owned(),
        ))
    }
}

/// Analytic first partial derivatives `(∂S/∂u, ∂S/∂v)` at `(u, v)`.
///
/// Rational B-spline derivatives are evaluated in homogeneous space and
/// projected with the quotient rule. This remains stable when valid imported
/// knot domains have large offsets that make finite-difference steps vanish.
pub fn partials(surface: &Surface, u: Scalar, v: Scalar) -> GeomResult<(Vec3, Vec3)> {
    finite(u, "surface parameter u")?;
    finite(v, "surface parameter v")?;
    finite_surface_frame(surface)?;
    let value = match surface {
        Surface::Plane(p) => Ok((p.frame.x, p.frame.y)),
        Surface::Cylinder(c) => {
            positive(c.radius, "cylinder radius")?;
            let (s, co) = u.sin_cos();
            Ok((
                direct(&c.frame, Vec3::new(-c.radius * s, c.radius * co, 0.0)),
                c.frame.z,
            ))
        }
        Surface::Cone(c) => {
            finite(c.radius, "cone radius")?;
            finite(c.semi_angle, "cone semi-angle")?;
            let slope = c.semi_angle.tan();
            let radius = c.radius + v * slope;
            if radius < 0.0 {
                return Err(GeomError::Degenerate(format!(
                    "cone radius is negative at v = {v}: the patch crosses the apex"
                )));
            }
            let (s, co) = u.sin_cos();
            Ok((
                direct(&c.frame, Vec3::new(-radius * s, radius * co, 0.0)),
                direct(&c.frame, Vec3::new(slope * co, slope * s, 1.0)),
            ))
        }
        Surface::Sphere(sphere) => {
            positive(sphere.radius, "sphere radius")?;
            let (su, cu) = u.sin_cos();
            let (sv, cv) = v.sin_cos();
            Ok((
                direct(
                    &sphere.frame,
                    Vec3::new(-sphere.radius * cv * su, sphere.radius * cv * cu, 0.0),
                ),
                direct(
                    &sphere.frame,
                    Vec3::new(
                        -sphere.radius * sv * cu,
                        -sphere.radius * sv * su,
                        sphere.radius * cv,
                    ),
                ),
            ))
        }
        Surface::Torus(torus) => {
            positive(torus.major_radius, "torus major radius")?;
            positive(torus.minor_radius, "torus minor radius")?;
            let (su, cu) = u.sin_cos();
            let (sv, cv) = v.sin_cos();
            let ring = torus.major_radius + torus.minor_radius * cv;
            Ok((
                direct(&torus.frame, Vec3::new(-ring * su, ring * cu, 0.0)),
                direct(
                    &torus.frame,
                    Vec3::new(
                        -torus.minor_radius * sv * cu,
                        -torus.minor_radius * sv * su,
                        torus.minor_radius * cv,
                    ),
                ),
            ))
        }
        Surface::BSpline(b) => bspline_partials(b, u, v),
        _ => Err(GeomError::Unsupported {
            backend: ScalarSurface::ID,
            operation: axiolid_kernel::Operation::SurfaceEvaluation,
        }),
    }?;
    if value.0.is_finite() && value.1.is_finite() {
        Ok(value)
    } else {
        Err(GeomError::Degenerate(
            "surface partial is non-finite".to_owned(),
        ))
    }
}

/// Unit normal at `(u, v)`.
///
/// Computed from the exact analytic partial derivatives rather than by
/// differencing evaluated points: a finite difference loses precision exactly
/// where it matters most, at high curvature.
pub fn normal(surface: &Surface, u: Scalar, v: Scalar) -> GeomResult<Vec3> {
    finite(u, "surface parameter u")?;
    finite(v, "surface parameter v")?;
    finite_surface_frame(surface)?;
    let n = match surface {
        Surface::Plane(p) => p.frame.z,
        Surface::Cylinder(c) => {
            positive(c.radius, "cylinder radius")?;
            let (s, co) = u.sin_cos();
            direct(&c.frame, Vec3::new(co, s, 0.0))
        }
        Surface::Cone(c) => cone_normal(c, u)?,
        Surface::Sphere(s) => {
            positive(s.radius, "sphere radius")?;
            let (su, cu) = u.sin_cos();
            let (sv, cv) = v.sin_cos();
            direct(&s.frame, Vec3::new(cv * cu, cv * su, sv))
        }
        Surface::Torus(t) => {
            positive(t.minor_radius, "torus minor radius")?;
            let (su, cu) = u.sin_cos();
            let (sv, cv) = v.sin_cos();
            direct(&t.frame, Vec3::new(cv * cu, cv * su, sv))
        }
        Surface::BSpline(b) => bspline_normal(b, u, v)?,
        _ => {
            return Err(GeomError::Unsupported {
                backend: ScalarSurface::ID,
                operation: axiolid_kernel::Operation::SurfaceEvaluation,
            })
        }
    };
    let length = n.length();
    if !(length > 0.0 && n.is_finite()) {
        return Err(GeomError::Degenerate(format!(
            "surface normal is not orientable at ({u}, {v})"
        )));
    }
    Ok(n / length)
}

fn plane_point(p: &Plane, u: Scalar, v: Scalar) -> Point3 {
    place(&p.frame, Vec3::new(u, v, 0.0))
}

fn cylinder_point(c: &Cylinder, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    positive(c.radius, "cylinder radius")?;
    let (s, co) = u.sin_cos();
    Ok(place(&c.frame, Vec3::new(c.radius * co, c.radius * s, v)))
}

fn cone_point(c: &Cone, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    finite(c.radius, "cone radius")?;
    finite(c.semi_angle, "cone semi-angle")?;
    // Radius shrinks with height at the semi-angle; a negative radius means
    // the surface has passed through the apex, which is not a valid patch.
    let r = c.radius + v * c.semi_angle.tan();
    if r < 0.0 {
        return Err(GeomError::Degenerate(format!(
            "cone radius is negative at v = {v}: the patch crosses the apex"
        )));
    }
    let (s, co) = u.sin_cos();
    Ok(place(&c.frame, Vec3::new(r * co, r * s, v)))
}

fn cone_normal(c: &Cone, u: Scalar) -> GeomResult<Vec3> {
    finite(c.semi_angle, "cone semi-angle")?;
    let (s, co) = u.sin_cos();
    // Outward radial component, tilted by the semi-angle: the normal leans
    // toward the axis as the cone narrows.
    let (sa, ca) = c.semi_angle.sin_cos();
    Ok(direct(&c.frame, Vec3::new(ca * co, ca * s, -sa)))
}

fn sphere_point(s: &Sphere, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    positive(s.radius, "sphere radius")?;
    let (su, cu) = u.sin_cos();
    let (sv, cv) = v.sin_cos();
    Ok(place(
        &s.frame,
        Vec3::new(s.radius * cv * cu, s.radius * cv * su, s.radius * sv),
    ))
}

fn torus_point(t: &Torus, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    positive(t.major_radius, "torus major radius")?;
    positive(t.minor_radius, "torus minor radius")?;
    let (su, cu) = u.sin_cos();
    let (sv, cv) = v.sin_cos();
    let ring = t.major_radius + t.minor_radius * cv;
    Ok(place(
        &t.frame,
        Vec3::new(ring * cu, ring * su, t.minor_radius * sv),
    ))
}

// --- tensor-product B-spline ------------------------------------------------

type Axis = SplineAxis;

/// Validate the control net and both axes together.
fn bspline_axes(b: &BSplineSurface) -> GeomResult<(Axis, Axis)> {
    let rows = b.control_points.len();
    if rows == 0 {
        return Err(GeomError::InvalidInput(
            "B-spline surface has no control points".to_owned(),
        ));
    }
    let cols = b.control_points[0].len();
    if cols == 0 {
        return Err(GeomError::InvalidInput(
            "B-spline surface control net has an empty row".to_owned(),
        ));
    }
    // A ragged net is a data error, not something to paper over: evaluating it
    // would silently read a different surface than the source declared.
    if b.control_points.iter().any(|row| row.len() != cols) {
        return Err(GeomError::InvalidInput(
            "B-spline surface control net is ragged".to_owned(),
        ));
    }
    if b.control_points
        .iter()
        .flatten()
        .any(|point| !point.is_finite())
    {
        return Err(GeomError::InvalidInput(
            "B-spline surface control points must be finite".to_owned(),
        ));
    }
    if let Some(w) = &b.weights {
        if w.len() != rows || w.iter().any(|row| row.len() != cols) {
            return Err(GeomError::InvalidInput(
                "B-spline surface weight net does not match the control net".to_owned(),
            ));
        }
        if w.iter()
            .flatten()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
        {
            return Err(GeomError::InvalidInput(
                "B-spline surface weights must be finite and strictly positive".to_owned(),
            ));
        }
    }
    let u = Axis::new(&b.u_knots, &b.u_multiplicities, b.u_degree, rows, "u")?;
    let v = Axis::new(&b.v_knots, &b.v_multiplicities, b.v_degree, cols, "v")?;
    Ok((u, v))
}

/// Tensor-product de Boor: evaluate along `v` per influencing row, then along
/// `u` through those results.
///
/// Rational surfaces interpolate in homogeneous space throughout; projecting
/// per row and averaging afterwards is the classic wrong answer.
fn bspline_point(b: &BSplineSurface, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    let (ua, va) = bspline_axes(b)?;
    let (uc, vc) = (ua.clamp(u), va.clamp(v));
    let uspan = span_in(&ua.knots, ua.count, ua.degree, uc);
    let vspan = span_in(&va.knots, va.count, va.degree, vc);

    // Stage one: collapse each influencing row along v, staying homogeneous.
    let mut row_points: Vec<[Scalar; 3]> = Vec::with_capacity(ua.degree + 1);
    let mut row_weights: Vec<Scalar> = Vec::with_capacity(ua.degree + 1);
    for i in 0..=ua.degree {
        let row = uspan - ua.degree + i;
        let mut pts: Vec<[Scalar; 3]> = Vec::with_capacity(va.degree + 1);
        let mut wts: Vec<Scalar> = Vec::with_capacity(va.degree + 1);
        for j in 0..=va.degree {
            let col = vspan - va.degree + j;
            let w = b.weights.as_ref().map_or(1.0, |ws| ws[row][col]);
            let p = b.control_points[row][col];
            let homogeneous = [p.x * w, p.y * w, p.z * w];
            if homogeneous.iter().any(|value| !value.is_finite()) {
                return Err(GeomError::Degenerate(
                    "B-spline surface homogeneous control point overflowed".to_owned(),
                ));
            }
            pts.push(homogeneous);
            wts.push(w);
        }
        de_boor_recurrence(&va.knots, vspan, va.degree, vc, &mut pts, &mut wts);
        row_points.push(pts[va.degree]);
        row_weights.push(wts[va.degree]);
    }

    // Stage two: collapse the row results along u.
    de_boor_recurrence(
        &ua.knots,
        uspan,
        ua.degree,
        uc,
        &mut row_points,
        &mut row_weights,
    );

    let w = row_weights[ua.degree];
    if !w.is_finite() || w == 0.0 {
        return Err(GeomError::Degenerate(
            "B-spline surface weight collapsed to zero".to_owned(),
        ));
    }
    let p = row_points[ua.degree];
    Ok(Point3::new(p[0] / w, p[1] / w, p[2] / w))
}

/// Borrowed knot axes for one homogeneous tensor-product evaluation.
#[derive(Clone, Copy)]
struct HomogeneousAxes<'a> {
    u_knots: &'a [Scalar],
    u_degree: usize,
    v_knots: &'a [Scalar],
    v_degree: usize,
}

/// Evaluate one homogeneous tensor-product control net without projecting.
fn eval_tensor_homogeneous(
    axes: HomogeneousAxes<'_>,
    points: &[Vec<[Scalar; 3]>],
    weights: &[Vec<Scalar>],
    u: Scalar,
    v: Scalar,
) -> ([Scalar; 3], Scalar) {
    let mut row_points = Vec::with_capacity(points.len());
    let mut row_weights = Vec::with_capacity(points.len());
    for (row_points_h, row_weights_h) in points.iter().zip(weights) {
        let (point, weight) =
            eval_homogeneous(axes.v_knots, axes.v_degree, row_points_h, row_weights_h, v);
        row_points.push(point);
        row_weights.push(weight);
    }
    eval_homogeneous(axes.u_knots, axes.u_degree, &row_points, &row_weights, u)
}

type HomogeneousPointNet = Vec<Vec<[Scalar; 3]>>;
type HomogeneousWeightNet = Vec<Vec<Scalar>>;

/// Homogeneous point and weight control nets for a validated surface.
fn homogeneous_control_net(
    b: &BSplineSurface,
) -> GeomResult<(HomogeneousPointNet, HomogeneousWeightNet)> {
    let mut points = Vec::with_capacity(b.control_points.len());
    let mut weights = Vec::with_capacity(b.control_points.len());
    for (i, row) in b.control_points.iter().enumerate() {
        let mut point_row = Vec::with_capacity(row.len());
        let mut weight_row = Vec::with_capacity(row.len());
        for (j, point) in row.iter().enumerate() {
            let weight = b.weights.as_ref().map_or(1.0, |net| net[i][j]);
            let homogeneous = [point.x * weight, point.y * weight, point.z * weight];
            if homogeneous.iter().any(|value| !value.is_finite()) {
                return Err(GeomError::Degenerate(
                    "B-spline surface homogeneous control point overflowed".to_owned(),
                ));
            }
            point_row.push(homogeneous);
            weight_row.push(weight);
        }
        points.push(point_row);
        weights.push(weight_row);
    }
    Ok((points, weights))
}

/// Differentiate a homogeneous control net along `u`.
fn derivative_net_u(
    points: &[Vec<[Scalar; 3]>],
    weights: &[Vec<Scalar>],
    knots: &[Scalar],
    degree: usize,
) -> (Vec<Vec<[Scalar; 3]>>, Vec<Vec<Scalar>>) {
    let rows = points.len() - 1;
    let cols = points[0].len();
    let mut derivative_points = Vec::with_capacity(rows);
    let mut derivative_weights = Vec::with_capacity(rows);
    for i in 0..rows {
        let denominator = knots[i + degree + 1] - knots[i + 1];
        let factor = if denominator.abs() > 0.0 {
            degree as Scalar / denominator
        } else {
            0.0
        };
        let mut point_row = Vec::with_capacity(cols);
        let mut weight_row = Vec::with_capacity(cols);
        for j in 0..cols {
            point_row.push(core::array::from_fn(|k| {
                factor * (points[i + 1][j][k] - points[i][j][k])
            }));
            weight_row.push(factor * (weights[i + 1][j] - weights[i][j]));
        }
        derivative_points.push(point_row);
        derivative_weights.push(weight_row);
    }
    (derivative_points, derivative_weights)
}

/// Differentiate a homogeneous control net along `v`.
fn derivative_net_v(
    points: &[Vec<[Scalar; 3]>],
    weights: &[Vec<Scalar>],
    knots: &[Scalar],
    degree: usize,
) -> (Vec<Vec<[Scalar; 3]>>, Vec<Vec<Scalar>>) {
    let rows = points.len();
    let cols = points[0].len() - 1;
    let mut derivative_points = Vec::with_capacity(rows);
    let mut derivative_weights = Vec::with_capacity(rows);
    for i in 0..rows {
        let mut point_row = Vec::with_capacity(cols);
        let mut weight_row = Vec::with_capacity(cols);
        for j in 0..cols {
            let denominator = knots[j + degree + 1] - knots[j + 1];
            let factor = if denominator.abs() > 0.0 {
                degree as Scalar / denominator
            } else {
                0.0
            };
            point_row.push(core::array::from_fn(|k| {
                factor * (points[i][j + 1][k] - points[i][j][k])
            }));
            weight_row.push(factor * (weights[i][j + 1] - weights[i][j]));
        }
        derivative_points.push(point_row);
        derivative_weights.push(weight_row);
    }
    (derivative_points, derivative_weights)
}

/// Project one homogeneous derivative with the rational quotient rule.
fn project_derivative(
    point: [Scalar; 3],
    weight: Scalar,
    derivative: [Scalar; 3],
    derivative_weight: Scalar,
    axis: &str,
) -> GeomResult<Vec3> {
    if !weight.is_finite() || weight == 0.0 {
        return Err(GeomError::Degenerate(
            "B-spline surface weight collapsed to a non-finite or zero value".to_owned(),
        ));
    }
    let value = Vec3::new(
        (derivative[0] - point[0] * derivative_weight / weight) / weight,
        (derivative[1] - point[1] * derivative_weight / weight) / weight,
        (derivative[2] - point[2] * derivative_weight / weight) / weight,
    );
    if !value.is_finite() {
        return Err(GeomError::Degenerate(format!(
            "B-spline surface {axis} derivative is non-finite"
        )));
    }
    Ok(value)
}

/// Exact first partials of a rational tensor-product B-spline.
fn bspline_partials(b: &BSplineSurface, u: Scalar, v: Scalar) -> GeomResult<(Vec3, Vec3)> {
    let (ua, va) = bspline_axes(b)?;
    let (uc, vc) = (ua.clamp(u), va.clamp(v));
    let (points, weights) = homogeneous_control_net(b)?;
    let (point, weight) = eval_tensor_homogeneous(
        HomogeneousAxes {
            u_knots: &ua.knots,
            u_degree: ua.degree,
            v_knots: &va.knots,
            v_degree: va.degree,
        },
        &points,
        &weights,
        uc,
        vc,
    );

    let (u_points, u_weights) = derivative_net_u(&points, &weights, &ua.knots, ua.degree);
    let (du, du_weight) = eval_tensor_homogeneous(
        HomogeneousAxes {
            u_knots: &ua.knots[1..ua.knots.len() - 1],
            u_degree: ua.degree - 1,
            v_knots: &va.knots,
            v_degree: va.degree,
        },
        &u_points,
        &u_weights,
        uc,
        vc,
    );

    let (v_points, v_weights) = derivative_net_v(&points, &weights, &va.knots, va.degree);
    let (dv, dv_weight) = eval_tensor_homogeneous(
        HomogeneousAxes {
            u_knots: &ua.knots,
            u_degree: ua.degree,
            v_knots: &va.knots[1..va.knots.len() - 1],
            v_degree: va.degree - 1,
        },
        &v_points,
        &v_weights,
        uc,
        vc,
    );

    Ok((
        project_derivative(point, weight, du, du_weight, "u")?,
        project_derivative(point, weight, dv, dv_weight, "v")?,
    ))
}

/// Normal from analytic rational tensor-product partial derivatives.
fn bspline_normal(b: &BSplineSurface, u: Scalar, v: Scalar) -> GeomResult<Vec3> {
    let (du, dv) = bspline_partials(b, u, v)?;
    Ok(du.cross(dv))
}

/// The [`axiolid_surface::SurfaceEvaluator`] implementation, so a caller can dispatch through
/// the trait rather than the free functions.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScalarSurface;

impl ScalarSurface {
    /// Identity reported in structured errors, matching `ScalarBoolean`.
    pub const ID: BackendId = BackendId::new("scalar-reference");
}

impl axiolid_surface::SurfaceEvaluator<Surface> for ScalarSurface {
    type Error = GeomError;

    fn evaluate(
        &self,
        surface: &Surface,
        u: Scalar,
        v: Scalar,
        _tolerance: axiolid_core::Tolerance,
    ) -> Result<Point3, Self::Error> {
        evaluate(surface, u, v)
    }

    fn normal(
        &self,
        surface: &Surface,
        u: Scalar,
        v: Scalar,
        _tolerance: axiolid_core::Tolerance,
    ) -> Result<Vec3, Self::Error> {
        normal(surface, u, v)
    }
}

/// Surface parameters `(u, v)` whose evaluation reproduces `point`.
///
/// This is the exact inverse of [`evaluate`] for the analytic surfaces,
/// derived from each parameterisation rather than found by iteration, so
/// it neither needs a seed nor converges to a nearby-but-wrong branch.
///
/// The point must already lie ON the surface: this answers "which
/// parameters name this point", not "which point is nearest". A sweep
/// directrix that has drifted off its reference surface is a modelling
/// error, and silently projecting it would tilt every section frame by an
/// amount nothing downstream can detect. The residual is therefore checked
/// against `tolerance` and a miss is reported rather than absorbed.
///
/// Parameters that no unique answer exists for are refused, not guessed:
/// at a cone apex or a sphere pole the whole `u` circle maps to one point,
/// so any choice would be arbitrary and would rotate the swept section.
pub fn invert(
    surface: &Surface,
    point: Point3,
    tolerance: axiolid_core::Tolerance,
) -> GeomResult<(Scalar, Scalar)> {
    let (u, v) = match surface {
        Surface::Plane(p) => {
            let local = to_local(&p.frame, point)?;
            (local.x, local.y)
        }
        Surface::Cylinder(c) => {
            positive(c.radius, "cylinder radius")?;
            let local = to_local(&c.frame, point)?;
            (angle_about_axis(local, "cylinder")?, local.z)
        }
        Surface::Cone(c) => {
            finite(c.radius, "cone radius")?;
            finite(c.semi_angle, "cone semi-angle")?;
            let local = to_local(&c.frame, point)?;
            // At the apex the radius vanishes and every u names the same
            // point, so the angle is unrecoverable rather than merely
            // imprecise.
            (angle_about_axis(local, "cone")?, local.z)
        }
        Surface::Sphere(s) => {
            positive(s.radius, "sphere radius")?;
            let local = to_local(&s.frame, point)?;
            // Latitude first: it is well defined even at the poles, which
            // the angle lookup then rejects.
            let sin_v = (local.z / s.radius).clamp(-1.0, 1.0);
            (angle_about_axis(local, "sphere")?, sin_v.asin())
        }
        Surface::Torus(t) => {
            positive(t.major_radius, "torus major radius")?;
            positive(t.minor_radius, "torus minor radius")?;
            let local = to_local(&t.frame, point)?;
            let ring = (local.x * local.x + local.y * local.y).sqrt();
            (
                angle_about_axis(local, "torus")?,
                (local.z).atan2(ring - t.major_radius),
            )
        }
        // A B-spline has no closed-form inverse; recovering parameters
        // needs iterative closest-point with its own seeding and
        // convergence contract. `Surface` is non-exhaustive, so any
        // future variant lands here too and is refused by name rather
        // than silently taking an analytic branch that does not fit it.
        _ => {
            return Err(GeomError::Unsupported {
                backend: ScalarSurface::ID,
                operation: axiolid_kernel::Operation::SurfaceEvaluation,
            });
        }
    };
    // The parameters are only meaningful if they reproduce the point.
    // This is what turns a silent mis-parameterisation into an error.
    let round_trip = evaluate(surface, u, v)?;
    let residual = (round_trip - point).length();
    if residual > tolerance.linear() {
        return Err(GeomError::Degenerate(format!(
            "point is {residual} from the surface, beyond the {} tolerance: \
             inversion names a point ON the surface and does not project",
            tolerance.linear()
        )));
    }
    Ok((u, v))
}

/// Express a world point in a frame's local coordinates.
///
/// `place` maps local to world by scaling the frame axes, so the inverse
/// is a projection onto those axes -- but ONLY when they are orthonormal.
/// `Frame3` stores three free vectors and the core documents that
/// algorithms validate orthonormality explicitly, so this checks rather
/// than assumes: on a skewed or scaled frame a dot-product projection is
/// silently wrong, and every parameter derived from it would be wrong by
/// an amount that still round-trips through the same bad frame.
fn to_local(frame: &Frame3, point: Point3) -> GeomResult<Vec3> {
    let axes = [frame.x, frame.y, frame.z];
    for (axis, name) in axes.iter().zip(["x", "y", "z"]) {
        let length = axis.length();
        if (length - 1.0).abs() > 1e-9 {
            return Err(GeomError::Degenerate(format!(
                "surface frame {name} axis has length {length}, expected 1"
            )));
        }
    }
    for (a, b, pair) in [
        (frame.x, frame.y, "x/y"),
        (frame.y, frame.z, "y/z"),
        (frame.z, frame.x, "z/x"),
    ] {
        let dot = a.dot(b);
        if dot.abs() > 1e-9 {
            return Err(GeomError::Degenerate(format!(
                "surface frame {pair} axes are not perpendicular: dot {dot}"
            )));
        }
    }
    let offset = point - frame.origin;
    Ok(Vec3::new(
        offset.dot(frame.x),
        offset.dot(frame.y),
        offset.dot(frame.z),
    ))
}

/// Angle of a local point about the frame's z axis.
///
/// Refuses points ON the axis. There the whole `u` circle collapses to a
/// single location -- a cone apex, a sphere pole -- so no angle is more
/// correct than any other. Returning zero would look successful and would
/// rotate a swept section arbitrarily about its own path.
fn angle_about_axis(local: Vec3, surface: &str) -> GeomResult<Scalar> {
    let radial = (local.x * local.x + local.y * local.y).sqrt();
    if radial <= 1e-12 {
        return Err(GeomError::Degenerate(format!(
            "{surface} point lies on the axis, where every u names it: \
             the angular parameter is not recoverable"
        )));
    }
    Ok(local.y.atan2(local.x))
}
