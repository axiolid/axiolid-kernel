//! Bounded inverse queries for NURBS curves.

use crate::axis::active_spans;
use crate::projection::{CurveProjection2, CurveProjection3, ProjectionOptions, ProjectionStatus};
use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{BSplineCurve2, BSplineCurve3};
use axiolid_reference::curve::{bspline_jet2, bspline_jet3};

/// Find the best planar-curve projection candidate within explicit budgets.
///
/// Every active knot span and both domain endpoints are seeded. The result is
/// not a certified global minimum; inspect its status and distance.
pub fn project_curve2(
    curve: &BSplineCurve2,
    target: Point2,
    options: ProjectionOptions,
) -> GeomResult<CurveProjection2> {
    let spans = active_spans(
        &curve.knots,
        &curve.multiplicities,
        curve.degree,
        curve.control_points.len(),
    )?;
    let (t, p, d, it, status, on_boundary) = project([target.x, target.y], &spans, options, |t| {
        let j = bspline_jet2(curve, t)?;
        Ok((
            [j.point.x, j.point.y],
            [j.first.x, j.first.y],
            [j.second.x, j.second.y],
        ))
    })?;
    Ok(CurveProjection2 {
        parameter: t,
        point: Point2::new(p[0], p[1]),
        distance: d,
        iterations: it,
        on_boundary,
        status,
    })
}

/// Find the best spatial-curve projection candidate within explicit budgets.
///
/// Every active knot span and both domain endpoints are seeded. The result is
/// not a certified global minimum; inspect its status and distance.
pub fn project_curve3(
    curve: &BSplineCurve3,
    target: Point3,
    options: ProjectionOptions,
) -> GeomResult<CurveProjection3> {
    let spans = active_spans(
        &curve.knots,
        &curve.multiplicities,
        curve.degree,
        curve.control_points.len(),
    )?;
    let (t, p, d, it, status, on_boundary) =
        project([target.x, target.y, target.z], &spans, options, |t| {
            let j = bspline_jet3(curve, t)?;
            Ok((
                [j.point.x, j.point.y, j.point.z],
                [j.first.x, j.first.y, j.first.z],
                [j.second.x, j.second.y, j.second.z],
            ))
        })?;
    Ok(CurveProjection3 {
        parameter: t,
        point: Point3::new(p[0], p[1], p[2]),
        distance: d,
        iterations: it,
        on_boundary,
        status,
    })
}

type Candidate<const N: usize> = (Scalar, [Scalar; N], Scalar, u16, ProjectionStatus, bool);

fn project<const N: usize>(
    target: [Scalar; N],
    spans: &[(Scalar, Scalar)],
    options: ProjectionOptions,
    jet: impl Fn(Scalar) -> GeomResult<([Scalar; N], [Scalar; N], [Scalar; N])>,
) -> GeomResult<Candidate<N>> {
    if target.iter().any(|x| !x.is_finite()) {
        return Err(GeomError::InvalidInput(
            "projection target must be finite".to_owned(),
        ));
    }
    let lo = spans[0].0;
    let hi = spans[spans.len() - 1].1;
    let mut best: Option<Candidate<N>> = None;
    let mut starts = 0_u32;
    for &(a, b) in spans {
        for sample in 0..=options.samples_per_span() {
            starts = starts.checked_add(1).ok_or(GeomError::BudgetExceeded {
                resource: "projection starts",
            })?;
            if starts > options.max_starts() {
                return Err(GeomError::BudgetExceeded {
                    resource: "projection starts",
                });
            }
            let start =
                a + (b - a) * Scalar::from(sample) / Scalar::from(options.samples_per_span());
            let candidate = refine(target, start, lo, hi, options, &jet)?;
            if best.as_ref().is_none_or(|current| candidate.2 < current.2) {
                best = Some(candidate);
            }
        }
    }
    best.ok_or_else(|| GeomError::Degenerate("projection produced no candidate".to_owned()))
}

fn refine<const N: usize>(
    target: [Scalar; N],
    start: Scalar,
    lo: Scalar,
    hi: Scalar,
    options: ProjectionOptions,
    jet: &impl Fn(Scalar) -> GeomResult<([Scalar; N], [Scalar; N], [Scalar; N])>,
) -> GeomResult<Candidate<N>> {
    let mut t = start;
    let mut iterations = 0;
    let mut status = ProjectionStatus::BudgetExhausted;
    for iteration in 0..options.max_iterations() {
        iterations = iteration + 1;
        let (p, d1, d2) = jet(t)?;
        let r = sub(p, target);
        let speed = dot(d1, d1).sqrt();
        let gradient = dot(r, d1);
        if gradient.abs() <= options.tolerance().linear() * speed.max(1.0) {
            status = ProjectionStatus::Converged;
            break;
        }
        let hessian = dot(d1, d1) + dot(r, d2);
        if !hessian.is_finite() || hessian.abs() <= Scalar::EPSILON * dot(d1, d1).max(1.0) {
            break;
        }
        let next = (t - gradient / hessian).clamp(lo, hi);
        if (next - t).abs() * speed <= options.tolerance().linear() {
            t = next;
            status = ProjectionStatus::Converged;
            break;
        }
        t = next;
    }
    let (point, _, _) = jet(t)?;
    let distance = dot(sub(point, target), sub(point, target)).sqrt();
    if !distance.is_finite() {
        return Err(GeomError::Degenerate(
            "projection distance is non-finite".to_owned(),
        ));
    }
    Ok((t, point, distance, iterations, status, t == lo || t == hi))
}

fn sub<const N: usize>(a: [Scalar; N], b: [Scalar; N]) -> [Scalar; N] {
    core::array::from_fn(|i| a[i] - b[i])
}
fn dot<const N: usize>(a: [Scalar; N], b: [Scalar; N]) -> Scalar {
    (0..N).map(|i| a[i] * b[i]).sum()
}
