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

use crate::curve::{de_boor_recurrence, span_in};

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

/// Position on a surface at `(u, v)`.
pub fn evaluate(surface: &Surface, u: Scalar, v: Scalar) -> GeomResult<Point3> {
    finite(u, "surface parameter u")?;
    finite(v, "surface parameter v")?;
    match surface {
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

/// Flatten `(knots, multiplicities)` into the vector de Boor needs.
fn expand(knots: &[Scalar], multiplicities: &[u32]) -> Vec<Scalar> {
    let mut out = Vec::new();
    for (value, count) in knots.iter().zip(multiplicities.iter()) {
        for _ in 0..*count {
            out.push(*value);
        }
    }
    out
}

/// Validated per-axis setup: flat knots, control count, degree.
struct Axis {
    knots: Vec<Scalar>,
    count: usize,
    degree: usize,
}

impl Axis {
    fn new(
        knots: &[Scalar],
        multiplicities: &[u32],
        degree: u16,
        count: usize,
        label: &str,
    ) -> GeomResult<Self> {
        let degree = degree as usize;
        if degree == 0 {
            return Err(GeomError::InvalidInput(format!(
                "B-spline surface {label} degree must be at least 1"
            )));
        }
        let flat = expand(knots, multiplicities);
        if flat.len() != count + degree + 1 {
            return Err(GeomError::InvalidInput(format!(
                "B-spline surface {label} knot vector has {} entries, expected {}",
                flat.len(),
                count + degree + 1
            )));
        }
        if !matches!(
            flat[count].partial_cmp(&flat[degree]),
            Some(core::cmp::Ordering::Greater)
        ) {
            return Err(GeomError::Degenerate(format!(
                "B-spline surface {label} domain is empty"
            )));
        }
        Ok(Self {
            knots: flat,
            count,
            degree,
        })
    }

    fn clamp(&self, t: Scalar) -> Scalar {
        t.clamp(self.knots[self.degree], self.knots[self.count])
    }
}

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
    if let Some(w) = &b.weights {
        if w.len() != rows || w.iter().any(|row| row.len() != cols) {
            return Err(GeomError::InvalidInput(
                "B-spline surface weight net does not match the control net".to_owned(),
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
            pts.push([p.x * w, p.y * w, p.z * w]);
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
    if w.abs() <= 0.0 {
        return Err(GeomError::Degenerate(
            "B-spline surface weight collapsed to zero".to_owned(),
        ));
    }
    let p = row_points[ua.degree];
    Ok(Point3::new(p[0] / w, p[1] / w, p[2] / w))
}

/// Normal from central differences of the tensor-product evaluation.
///
/// Unlike the elementary families there is no closed form here without a
/// second de Boor pass for each partial; a symmetric difference at a step
/// scaled to the domain is accurate to `O(h^2)` and cannot silently produce a
/// wrong direction, because a collapsed cross product is reported as an error.
fn bspline_normal(b: &BSplineSurface, u: Scalar, v: Scalar) -> GeomResult<Vec3> {
    let (ua, va) = bspline_axes(b)?;
    let (u0, u1) = (ua.knots[ua.degree], ua.knots[ua.count]);
    let (v0, v1) = (va.knots[va.degree], va.knots[va.count]);
    let hu = (u1 - u0) * 1e-6;
    let hv = (v1 - v0) * 1e-6;
    let uc = ua.clamp(u);
    let vc = va.clamp(v);

    let du = bspline_point(b, (uc + hu).min(u1), vc)? - bspline_point(b, (uc - hu).max(u0), vc)?;
    let dv = bspline_point(b, uc, (vc + hv).min(v1))? - bspline_point(b, uc, (vc - hv).max(v0))?;
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
