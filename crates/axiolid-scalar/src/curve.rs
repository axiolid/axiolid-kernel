//! Scalar reference implementation of curve evaluation (ADR 0012).
//!
//! # What this closes
//!
//! `axiolid-curve` declares `Curve2`/`Curve3` and a `CurveEvaluator` trait. Until
//! now nothing in the workspace implemented that trait, so every declared curve
//! family was inert data. `axiolid-compile` worked around this with its own
//! private circle flattener and refused ellipses and B-splines outright.
//!
//! # Design
//!
//! Evaluation is analytic per family, never a generic subdivision fallback:
//!
//! - `Line`     -- `origin + t * direction`, exact.
//! - `Circle`   -- `origin + r*(cos t * x + sin t * y)`, `t` in radians.
//! - `Ellipse`  -- same with independent semi-axes. Note `t` is the
//!   *parametric* angle, not the polar angle; they differ except on axis.
//! - `Polyline` -- `t` in `[0, n)`, integer part selects the segment. Chosen
//!   over arc-length parameterization because it is exact and stable under
//!   degenerate (zero-length) segments, which imported data contains.
//! - `BSpline`  -- de Boor. Rational curves evaluate in homogeneous space and
//!   project, which is the only way to get correct rational derivatives.
//!
//! Derivatives are closed-form. A finite-difference derivative would make the
//! curvature oracle in `tests/curve.rs` self-referential: it would be checking
//! a difference quotient against a difference quotient.
//!
//! # Frames are used as given
//!
//! Imported frames may be non-orthonormal. Evaluation applies the frame axes as
//! written rather than orthonormalizing, so a caller sees the geometry its
//! source actually declared. Validation is a separate concern (`axiolid-heal`).

use axiolid_core::{Frame2, Frame3, Interval, Point2, Point3, Scalar, Tolerance, Vec2, Vec3};
use axiolid_curve::{
    BSplineCurve, Circle2, Circle3, Curve2, Curve3, CurveEvaluator, Ellipse2, Ellipse3, Line2,
    Line3, Polyline2, Polyline3,
};
use axiolid_kernel::{GeomError, GeomResult};

use crate::nurbs::SplineAxis;

/// Portable scalar curve evaluator.
///
/// Stateless: every method is a pure function of its arguments, so one instance
/// is freely shareable across threads.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScalarCurve;

impl ScalarCurve {
    /// Construct the evaluator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// --- parameter domains ------------------------------------------------------

/// Domain of a 2D curve.
#[must_use]
pub fn domain2(curve: &Curve2) -> Interval {
    match curve {
        // A line is infinite; the unit interval is the conventional finite
        // window. Bounded use always arrives via `ProfileSegment::domain`.
        Curve2::Line(_) => Interval::UNIT,
        Curve2::Circle(_) | Curve2::Ellipse(_) => full_turn(),
        Curve2::Polyline(p) => polyline_domain(p.points.len(), p.closed),
        Curve2::BSpline(b) => spline_domain(b),
        // Unknown family: no domain is knowable, so claim none.
        _ => Interval {
            start: 0.0,
            end: 0.0,
        },
    }
}

/// Domain of a 3D curve.
#[must_use]
pub fn domain3(curve: &Curve3) -> Interval {
    match curve {
        Curve3::Line(_) => Interval::UNIT,
        Curve3::Circle(_) | Curve3::Ellipse(_) => full_turn(),
        Curve3::Polyline(p) => polyline_domain(p.points.len(), p.closed),
        Curve3::BSpline(b) => spline_domain(b),
        _ => Interval {
            start: 0.0,
            end: 0.0,
        },
    }
}

fn full_turn() -> Interval {
    Interval {
        start: 0.0,
        end: core::f64::consts::TAU,
    }
}

/// Polyline parameter runs `[0, segment_count]`.
fn polyline_domain(count: usize, closed: bool) -> Interval {
    let segments = if closed {
        count
    } else {
        count.saturating_sub(1)
    };
    Interval {
        start: 0.0,
        end: segments as Scalar,
    }
}

/// Domain of a validated B-spline axis. Invalid imported data reports an empty
/// domain through the infallible evaluator trait and is rejected by evaluation.
fn spline_domain<P>(b: &BSplineCurve<P>) -> Interval {
    SplineAxis::new(
        &b.knots,
        &b.multiplicities,
        b.degree,
        b.control_points.len(),
        "B-spline curve",
    )
    .map_or(
        Interval {
            start: 0.0,
            end: 0.0,
        },
        |axis| {
            let (start, end) = axis.domain();
            Interval { start, end }
        },
    )
}

// --- 2D evaluation ----------------------------------------------------------

/// Position on a 2D curve.
pub fn evaluate2(curve: &Curve2, t: Scalar) -> GeomResult<Point2> {
    finite(t)?;
    let value = match curve {
        Curve2::Line(l) => Ok(line_point(l.origin, l.direction, t)),
        Curve2::Circle(c) => Ok(conic_point2(&c.frame, c.radius, c.radius, t)),
        Curve2::Ellipse(e) => Ok(conic_point2(&e.frame, e.semi_axis_x, e.semi_axis_y, t)),
        Curve2::Polyline(p) => polyline_point(&p.points, p.closed, t),
        Curve2::BSpline(b) => de_boor(b, t, |p| [p.x, p.y], |c| Point2::new(c[0], c[1])),
        // `Curve*` is #[non_exhaustive]. An unknown family is refused by name
        // rather than approximated by whichever arm happens to be nearest.
        _ => Err(GeomError::Unsupported {
            backend: axiolid_kernel::BackendId::new("axiolid-scalar"),
            operation: axiolid_kernel::Operation::CurveEvaluation,
        }),
    }?;
    finite2(value, "curve point")
}

/// First derivative of a 2D curve.
pub fn derivative2(curve: &Curve2, t: Scalar) -> GeomResult<Vec2> {
    finite(t)?;
    let value = match curve {
        Curve2::Line(l) => Ok(l.direction),
        Curve2::Circle(c) => Ok(conic_tangent2(&c.frame, c.radius, c.radius, t)),
        Curve2::Ellipse(e) => Ok(conic_tangent2(&e.frame, e.semi_axis_x, e.semi_axis_y, t)),
        Curve2::Polyline(p) => polyline_tangent(&p.points, p.closed, t),
        Curve2::BSpline(b) => de_boor_derivative(b, t, |p| [p.x, p.y], |c| Vec2::new(c[0], c[1])),
        // `Curve*` is #[non_exhaustive]. An unknown family is refused by name
        // rather than approximated by whichever arm happens to be nearest.
        _ => Err(GeomError::Unsupported {
            backend: axiolid_kernel::BackendId::new("axiolid-scalar"),
            operation: axiolid_kernel::Operation::CurveEvaluation,
        }),
    }?;
    finite2(value, "curve derivative")
}

// --- 3D evaluation ----------------------------------------------------------

/// Position on a 3D curve.
pub fn evaluate3(curve: &Curve3, t: Scalar) -> GeomResult<Point3> {
    finite(t)?;
    let value = match curve {
        Curve3::Line(l) => Ok(line_point(l.origin, l.direction, t)),
        Curve3::Circle(c) => Ok(conic_point3(&c.frame, c.radius, c.radius, t)),
        Curve3::Ellipse(e) => Ok(conic_point3(&e.frame, e.semi_axis_x, e.semi_axis_y, t)),
        Curve3::Polyline(p) => polyline_point(&p.points, p.closed, t),
        Curve3::BSpline(b) => de_boor(b, t, |p| [p.x, p.y, p.z], |c| Point3::new(c[0], c[1], c[2])),
        // `Curve*` is #[non_exhaustive]. An unknown family is refused by name
        // rather than approximated by whichever arm happens to be nearest.
        _ => Err(GeomError::Unsupported {
            backend: axiolid_kernel::BackendId::new("axiolid-scalar"),
            operation: axiolid_kernel::Operation::CurveEvaluation,
        }),
    }?;
    finite3(value, "curve point")
}

/// First derivative of a 3D curve.
pub fn derivative3(curve: &Curve3, t: Scalar) -> GeomResult<Vec3> {
    finite(t)?;
    let value = match curve {
        Curve3::Line(l) => Ok(l.direction),
        Curve3::Circle(c) => Ok(conic_tangent3(&c.frame, c.radius, c.radius, t)),
        Curve3::Ellipse(e) => Ok(conic_tangent3(&e.frame, e.semi_axis_x, e.semi_axis_y, t)),
        Curve3::Polyline(p) => polyline_tangent(&p.points, p.closed, t),
        Curve3::BSpline(b) => {
            de_boor_derivative(b, t, |p| [p.x, p.y, p.z], |c| Vec3::new(c[0], c[1], c[2]))
        }
        // `Curve*` is #[non_exhaustive]. An unknown family is refused by name
        // rather than approximated by whichever arm happens to be nearest.
        _ => Err(GeomError::Unsupported {
            backend: axiolid_kernel::BackendId::new("axiolid-scalar"),
            operation: axiolid_kernel::Operation::CurveEvaluation,
        }),
    }?;
    finite3(value, "curve derivative")
}

// --- family kernels ---------------------------------------------------------

fn finite(t: Scalar) -> GeomResult<()> {
    if t.is_finite() {
        Ok(())
    } else {
        Err(GeomError::InvalidInput(format!(
            "curve parameter must be finite, got {t}"
        )))
    }
}

fn finite2(value: Vec2, what: &str) -> GeomResult<Vec2> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(GeomError::Degenerate(format!("{what} is non-finite")))
    }
}

fn finite3(value: Vec3, what: &str) -> GeomResult<Vec3> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(GeomError::Degenerate(format!("{what} is non-finite")))
    }
}

fn line_point<P>(origin: P, direction: P, t: Scalar) -> P
where
    P: core::ops::Add<Output = P> + core::ops::Mul<Scalar, Output = P>,
{
    origin + direction * t
}

fn conic_point2(frame: &Frame2, rx: Scalar, ry: Scalar, t: Scalar) -> Point2 {
    frame.origin + frame.x * (rx * t.cos()) + frame.y * (ry * t.sin())
}

fn conic_tangent2(frame: &Frame2, rx: Scalar, ry: Scalar, t: Scalar) -> Vec2 {
    frame.x * (-rx * t.sin()) + frame.y * (ry * t.cos())
}

fn conic_point3(frame: &Frame3, rx: Scalar, ry: Scalar, t: Scalar) -> Point3 {
    frame.origin + frame.x * (rx * t.cos()) + frame.y * (ry * t.sin())
}

fn conic_tangent3(frame: &Frame3, rx: Scalar, ry: Scalar, t: Scalar) -> Vec3 {
    frame.x * (-rx * t.sin()) + frame.y * (ry * t.cos())
}

/// Segment index and local fraction for a polyline parameter.
///
/// Returns `None` when the polyline cannot be evaluated at all.
fn polyline_span(count: usize, closed: bool, t: Scalar) -> Option<(usize, usize, Scalar)> {
    let segments = if closed {
        count
    } else {
        count.saturating_sub(1)
    };
    if count < 2 || segments == 0 {
        return None;
    }
    // Clamp into range: the endpoint t == segments is the final vertex, which
    // would otherwise index one past the last segment.
    let clamped = t.clamp(0.0, segments as Scalar);
    let mut index = clamped.floor() as usize;
    if index >= segments {
        index = segments - 1;
    }
    let local = clamped - index as Scalar;
    let next = (index + 1) % count;
    Some((index, next, local))
}

fn polyline_point<P>(points: &[P], closed: bool, t: Scalar) -> GeomResult<P>
where
    P: Copy
        + core::ops::Add<Output = P>
        + core::ops::Sub<Output = P>
        + core::ops::Mul<Scalar, Output = P>,
{
    let (i, j, local) = polyline_span(points.len(), closed, t).ok_or_else(|| {
        GeomError::Degenerate(format!(
            "polyline with {} points has no evaluable segment",
            points.len()
        ))
    })?;
    Ok(points[i] + (points[j] - points[i]) * local)
}

fn polyline_tangent<P>(points: &[P], closed: bool, t: Scalar) -> GeomResult<P>
where
    P: Copy + core::ops::Sub<Output = P>,
{
    let (i, j, _) = polyline_span(points.len(), closed, t).ok_or_else(|| {
        GeomError::Degenerate(format!(
            "polyline with {} points has no evaluable segment",
            points.len()
        ))
    })?;
    // Derivative w.r.t. the unit-per-segment parameter is the full edge vector.
    Ok(points[j] - points[i])
}

// --- reusable de Boor core (shared with `crate::surface`) -------------------

/// Locate the knot span for `u` in a validated flat knot vector.
///
/// Extracted from [`spline_span`] so a tensor-product surface can reuse the
/// exact same span logic per axis. `n` is the control-point count, `d` the
/// degree; the caller has already checked `knots.len() == n + d + 1`.
pub(crate) fn span_in(knots: &[Scalar], n: usize, d: usize, u: Scalar) -> usize {
    let mut span = d;
    for (k, knot) in knots.iter().enumerate().take(n).skip(d) {
        if *knot <= u {
            span = k;
        } else {
            break;
        }
    }
    span
}

/// One de Boor recurrence over homogeneous coordinates.
///
/// `points` holds the `d+1` premultiplied control points influencing `span`,
/// `weights` their weights. Both are consumed in place. This is the numerical
/// heart shared by curve and surface evaluation: keeping one copy means a fix
/// to the recurrence cannot land in one and not the other.
pub(crate) fn de_boor_recurrence<const N: usize>(
    knots: &[Scalar],
    span: usize,
    d: usize,
    u: Scalar,
    points: &mut [[Scalar; N]],
    weights: &mut [Scalar],
) {
    for r in 1..=d {
        for j in (r..=d).rev() {
            let i = span - d + j;
            let denom = knots[i + d + 1 - r] - knots[i];
            let alpha = if denom.abs() > 0.0 {
                (u - knots[i]) / denom
            } else {
                0.0
            };
            for k in 0..N {
                points[j][k] = points[j - 1][k] * (1.0 - alpha) + points[j][k] * alpha;
            }
            weights[j] = weights[j - 1] * (1.0 - alpha) + weights[j] * alpha;
        }
    }
}

// --- de Boor ----------------------------------------------------------------

/// Shared setup: validated flat knots, degree, and the knot span for `t`.
fn spline_span<P>(b: &BSplineCurve<P>, t: Scalar) -> GeomResult<(Vec<Scalar>, usize, usize)> {
    let axis = SplineAxis::new(
        &b.knots,
        &b.multiplicities,
        b.degree,
        b.control_points.len(),
        "B-spline curve",
    )?;
    if let Some(weights) = &b.weights {
        if weights.len() != b.control_points.len() {
            return Err(GeomError::InvalidInput(format!(
                "B-spline has {} weights for {} control points",
                weights.len(),
                b.control_points.len()
            )));
        }
        if weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
        {
            return Err(GeomError::InvalidInput(
                "B-spline weights must be finite and strictly positive".to_owned(),
            ));
        }
    }
    let t = axis.clamp(t);
    let span = span_in(&axis.knots, axis.count, axis.degree, t);
    Ok((axis.knots, span, axis.degree))
}

/// Convert and validate every control point before selecting a knot span.
/// Imported NaN/Inf coordinates must not be hidden in currently uninfluential
/// spans and surface later when the parameter changes.
fn finite_control_points<P, const N: usize, F>(
    control_points: &[P],
    to: &F,
) -> GeomResult<Vec<[Scalar; N]>>
where
    F: Fn(&P) -> [Scalar; N],
{
    let points: Vec<[Scalar; N]> = control_points.iter().map(to).collect();
    if points
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(GeomError::InvalidInput(
            "B-spline control points must be finite".to_owned(),
        ));
    }
    Ok(points)
}

/// Position via de Boor's algorithm.
///
/// `to` and `from` convert between the point type and a fixed-size coordinate
/// array so 2D and 3D share one implementation. Rational curves are evaluated
/// in homogeneous coordinates `(w*x, w*y, [w*z], w)` and projected at the end.
fn de_boor<P, const N: usize, F, G, Q>(
    b: &BSplineCurve<P>,
    t: Scalar,
    to: F,
    from: G,
) -> GeomResult<Q>
where
    F: Fn(&P) -> [Scalar; N],
    G: Fn([Scalar; N]) -> Q,
{
    let (knots, span, d) = spline_span(b, t)?;
    let control_points = finite_control_points(&b.control_points, &to)?;
    let u = t.clamp(knots[d], knots[b.control_points.len()]);

    // Working set: the d+1 control points influencing this span, in homogeneous
    // form. The trailing slot holds the weight (1.0 for polynomial curves).
    let mut work: Vec<[Scalar; N]> = Vec::with_capacity(d + 1);
    let mut weights: Vec<Scalar> = Vec::with_capacity(d + 1);
    for j in 0..=d {
        let idx = span - d + j;
        let w = b.weights.as_ref().map_or(1.0, |ws| ws[idx]);
        let c = control_points[idx];
        // Premultiply by w: interpolating in homogeneous space is what makes
        // rational curves correct. Projecting first would be plain averaging.
        let homogeneous = core::array::from_fn(|k| c[k] * w);
        if homogeneous.iter().any(|value| !value.is_finite()) {
            return Err(GeomError::Degenerate(
                "B-spline homogeneous control point overflowed".to_owned(),
            ));
        }
        work.push(homogeneous);
        weights.push(w);
    }

    // A repeated knot makes an interval empty; the shared recurrence treats
    // that as alpha = 0, which is the correct limit.
    de_boor_recurrence(&knots, span, d, u, &mut work, &mut weights);

    let w = weights[d];
    if !w.is_finite() || w == 0.0 {
        return Err(GeomError::Degenerate(
            "B-spline weight collapsed to zero".to_owned(),
        ));
    }
    Ok(from(core::array::from_fn(|k| work[d][k] / w)))
}

/// First derivative via the hodograph, with the quotient rule for rationals.
///
/// The derivative of a degree-`d` B-spline is a degree-`(d-1)` B-spline over
/// the same knots minus their outermost entries, with control points
/// `d * (P[i+1] - P[i]) / (knots[i+d+1] - knots[i+1])`.
///
/// For a rational curve `C = A/w`, both `A` and `w` are differentiated in
/// homogeneous space and combined as `(A' - C * w') / w`.
fn de_boor_derivative<P, const N: usize, F, G, Q>(
    b: &BSplineCurve<P>,
    t: Scalar,
    to: F,
    from: G,
) -> GeomResult<Q>
where
    F: Fn(&P) -> [Scalar; N],
    G: Fn([Scalar; N]) -> Q,
{
    let (knots, _, d) = spline_span(b, t)?;
    let control_points = finite_control_points(&b.control_points, &to)?;
    let n = b.control_points.len();
    let u = t.clamp(knots[d], knots[n]);

    // Homogeneous control points, weight in a parallel array.
    let hom: Vec<[Scalar; N]> = (0..n)
        .map(|i| {
            let w = b.weights.as_ref().map_or(1.0, |ws| ws[i]);
            let c = control_points[i];
            core::array::from_fn(|k| c[k] * w)
        })
        .collect();
    if hom.iter().flatten().any(|value| !value.is_finite()) {
        return Err(GeomError::Degenerate(
            "B-spline homogeneous control point overflowed".to_owned(),
        ));
    }
    let hw: Vec<Scalar> = (0..n)
        .map(|i| b.weights.as_ref().map_or(1.0, |ws| ws[i]))
        .collect();

    // Hodograph control points.
    let mut dhom: Vec<[Scalar; N]> = Vec::with_capacity(n - 1);
    let mut dhw: Vec<Scalar> = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let denom = knots[i + d + 1] - knots[i + 1];
        let f = if denom.abs() > 0.0 {
            d as Scalar / denom
        } else {
            0.0
        };
        dhom.push(core::array::from_fn(|k| (hom[i + 1][k] - hom[i][k]) * f));
        dhw.push((hw[i + 1] - hw[i]) * f);
    }

    // Evaluate the hodograph at u with degree d-1 over the trimmed knots.
    let dknots = &knots[1..knots.len() - 1];
    let (da, dw) = eval_homogeneous(dknots, d - 1, &dhom, &dhw, u);
    // Evaluate the curve itself for the quotient rule.
    let (a, w) = eval_homogeneous(&knots, d, &hom, &hw, u);

    if !w.is_finite() || w == 0.0 {
        return Err(GeomError::Degenerate(
            "B-spline weight collapsed to zero".to_owned(),
        ));
    }
    // C = A/w  =>  C' = (A' - (A/w) * w') / w
    Ok(from(core::array::from_fn(|k| {
        (da[k] - (a[k] / w) * dw) / w
    })))
}

/// de Boor over explicit homogeneous arrays; returns `(numerator, weight)`.
pub(crate) fn eval_homogeneous<const N: usize>(
    knots: &[Scalar],
    d: usize,
    hom: &[[Scalar; N]],
    hw: &[Scalar],
    u: Scalar,
) -> ([Scalar; N], Scalar) {
    let n = hom.len();
    if d == 0 {
        // Degree zero: piecewise constant, pick the containing span.
        let mut idx = 0;
        for (k, knot) in knots.iter().enumerate().take(n) {
            if *knot <= u {
                idx = k;
            }
        }
        return (hom[idx.min(n - 1)], hw[idx.min(n - 1)]);
    }
    let mut span = d;
    for (k, knot) in knots.iter().enumerate().take(n).skip(d) {
        if *knot <= u {
            span = k;
        } else {
            break;
        }
    }
    let mut work: Vec<[Scalar; N]> = (0..=d).map(|j| hom[span - d + j]).collect();
    let mut weights: Vec<Scalar> = (0..=d).map(|j| hw[span - d + j]).collect();
    for r in 1..=d {
        for j in (r..=d).rev() {
            let i = span - d + j;
            let denom = knots[i + d + 1 - r] - knots[i];
            let alpha = if denom.abs() > 0.0 {
                (u - knots[i]) / denom
            } else {
                0.0
            };
            for k in 0..N {
                work[j][k] = work[j - 1][k] * (1.0 - alpha) + work[j][k] * alpha;
            }
            weights[j] = weights[j - 1] * (1.0 - alpha) + weights[j] * alpha;
        }
    }
    (work[d], weights[d])
}

// --- adaptive flattening ----------------------------------------------------

/// Flatten a 2D curve over `domain` so the chord never deviates from the true
/// curve by more than `chord_tolerance`.
///
/// # Why bisection rather than a closed-form segment count
///
/// A count derived from radius and tolerance only works for circles. Bisecting
/// on measured sagitta works for every family, including rational splines whose
/// curvature varies along the span, and it degrades gracefully on the
/// degenerate inputs imported data actually contains.
///
/// The returned polyline includes both endpoints and is ordered along
/// increasing parameter. `max_depth` bounds the work: a caller gets a
/// deterministic result rather than an unbounded subdivision on a pathological
/// curve.
pub fn flatten2(
    curve: &Curve2,
    domain: Interval,
    chord_tolerance: Scalar,
    max_depth: u32,
) -> GeomResult<Vec<Point2>> {
    // A depth bound alone is not a resource bound: depth `d` permits `2^d`
    // segments. Cap the total point count too, so a curve that cannot meet
    // the tolerance fails fast instead of exhausting memory.
    const MAX_POINTS: usize = 1 << 16;
    if !(chord_tolerance.is_finite()
        && chord_tolerance.is_sign_positive()
        && chord_tolerance != 0.0)
    {
        return Err(GeomError::InvalidInput(format!(
            "chord tolerance must be positive and finite, got {chord_tolerance}"
        )));
    }
    // A line and a polyline are already exact between their breakpoints:
    // subdividing them adds vertices that carry no information.
    if let Curve2::Line(_) = curve {
        return Ok(vec![
            evaluate2(curve, domain.start)?,
            evaluate2(curve, domain.end)?,
        ]);
    }
    if let Curve2::Polyline(p) = curve {
        // A polyline's parameter is one unit per segment, so a caller passing
        // a normalized `(0, 1)` domain would silently collapse an n-vertex
        // ring to its first edge. That is data loss disguised as success, so
        // it is refused: a domain narrower than one segment can only be
        // intentional for a genuinely 1-segment polyline.
        let natural = polyline_domain(p.points.len(), p.closed);
        let requested = (domain.end - domain.start).abs();
        if natural.end > 1.0 && requested <= 1.0 {
            return Err(GeomError::InvalidInput(format!(
                "polyline domain {:?} spans {requested} of {} segments; a \
                 polyline parameter is one unit per segment, so this would \
                 discard {} vertices",
                domain,
                natural.end,
                p.points.len().saturating_sub(2)
            )));
        }
        return polyline_flatten(&p.points, p.closed, domain, |t| evaluate2(curve, t));
    }

    let mut out = vec![evaluate2(curve, domain.start)?];
    subdivide2(
        curve,
        domain.start,
        domain.end,
        chord_tolerance,
        max_depth.min(MAX_DEPTH_CEILING),
        MAX_POINTS,
        &mut out,
    )?;
    out.push(evaluate2(curve, domain.end)?);
    Ok(out)
}

/// Hard ceiling on recursion depth regardless of what a caller asks for.
///
/// 2^20 segments is already far past any usable tolerance; beyond this a
/// request is a bug, not a quality setting.
const MAX_DEPTH_CEILING: u32 = 20;

/// Emit interior points of `(a, b)` that are needed to meet the tolerance.
///
/// `budget` bounds total emitted points. Exceeding it is an error rather than
/// a truncation: silently returning a coarser polyline than the caller asked
/// for would violate the tolerance contract this function exists to honour.
fn subdivide2(
    curve: &Curve2,
    a: Scalar,
    b: Scalar,
    tol: Scalar,
    depth: u32,
    budget: usize,
    out: &mut Vec<Point2>,
) -> GeomResult<()> {
    if out.len() >= budget {
        return Err(GeomError::Degenerate(format!(
            "curve flattening exceeded {budget} points before meeting the \
             chord tolerance {tol}; the curve may be degenerate"
        )));
    }
    let mid = 0.5 * (a + b);
    // A parameter interval too small to bisect cannot be refined further:
    // `mid` equals `a` or `b` in floating point and the recursion would not
    // terminate on its own.
    if !(mid > a && mid < b) {
        return Err(GeomError::Degenerate(format!(
            "curve flattening cannot bisect parameter interval [{a}, {b}] before meeting chord tolerance {tol}"
        )));
    }
    let pa = evaluate2(curve, a)?;
    let pb = evaluate2(curve, b)?;
    let pm = evaluate2(curve, mid)?;
    if sagitta2(pa, pb, pm) <= tol {
        // Within tolerance: the chord a->b stands, no interior point.
        return Ok(());
    }
    if depth == 0 {
        return Err(GeomError::BudgetExceeded {
            resource: "curve flattening depth",
        });
    }
    subdivide2(curve, a, mid, tol, depth - 1, budget, out)?;
    out.push(pm);
    subdivide2(curve, mid, b, tol, depth - 1, budget, out)?;
    Ok(())
}

/// Distance from `m` to segment `a-b`: the sagitta of this subdivision step.
fn sagitta2(a: Point2, b: Point2, m: Point2) -> Scalar {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 0.0 {
        // Degenerate chord: fall back to point distance so a closed curve
        // whose endpoints coincide still subdivides.
        return (m - a).length();
    }
    let t = ((m - a).dot(ab) / len2).clamp(0.0, 1.0);
    (m - (a + ab * t)).length()
}

/// Polylines flatten to their own breakpoints, restricted to `domain`.
fn polyline_flatten<P, F>(
    points: &[P],
    closed: bool,
    domain: Interval,
    eval: F,
) -> GeomResult<Vec<P>>
where
    P: Copy,
    F: Fn(Scalar) -> GeomResult<P>,
{
    let segments = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    if segments == 0 {
        return Err(GeomError::Degenerate(
            "polyline has no evaluable segment".to_owned(),
        ));
    }
    let lo = domain.start.min(domain.end);
    let hi = domain.start.max(domain.end);
    let mut out = vec![eval(lo)?];
    // Interior breakpoints are the integer parameters strictly inside.
    let first = lo.floor() as i64 + 1;
    let last = hi.ceil() as i64 - 1;
    for k in first..=last {
        let t = k as Scalar;
        if t > lo && t < hi {
            out.push(eval(t)?);
        }
    }
    out.push(eval(hi)?);
    Ok(out)
}

// --- trait wiring -----------------------------------------------------------

impl CurveEvaluator<Curve2> for ScalarCurve {
    type Point = Point2;
    type Derivative = Vec2;
    type Error = GeomError;

    fn domain(&self, curve: &Curve2) -> Interval {
        domain2(curve)
    }

    fn evaluate(
        &self,
        curve: &Curve2,
        t: Scalar,
        _tolerance: Tolerance,
    ) -> Result<Self::Point, Self::Error> {
        evaluate2(curve, t)
    }

    fn derivative(
        &self,
        curve: &Curve2,
        t: Scalar,
        _tolerance: Tolerance,
    ) -> Result<Self::Derivative, Self::Error> {
        derivative2(curve, t)
    }
}

impl CurveEvaluator<Curve3> for ScalarCurve {
    type Point = Point3;
    type Derivative = Vec3;
    type Error = GeomError;

    fn domain(&self, curve: &Curve3) -> Interval {
        domain3(curve)
    }

    fn evaluate(
        &self,
        curve: &Curve3,
        t: Scalar,
        _tolerance: Tolerance,
    ) -> Result<Self::Point, Self::Error> {
        evaluate3(curve, t)
    }

    fn derivative(
        &self,
        curve: &Curve3,
        t: Scalar,
        _tolerance: Tolerance,
    ) -> Result<Self::Derivative, Self::Error> {
        derivative3(curve, t)
    }
}

// Silence unused-import warnings for types only named in signatures.
#[allow(unused)]
fn _type_anchors(_: Circle2, _: Circle3, _: Ellipse2, _: Ellipse3, _: Line2, _: Line3) {}
#[allow(unused)]
fn _poly_anchors(_: Polyline2, _: Polyline3) {}
