//! Bounded inverse queries for tensor-product NURBS surfaces.

use crate::axis::active_spans;
use crate::projection::{ProjectionOptions, ProjectionStatus, SurfaceProjection};
use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar};
use axiolid_evaluate::surface::bspline_jet;
use axiolid_surface::BSplineSurface;

/// Find the best tensor-product surface projection candidate within budgets.
///
/// The Cartesian product of active U/V knot-span seeds is refined with the
/// exact squared-distance Hessian. The result is not a certified global minimum.
pub fn project_surface(
    surface: &BSplineSurface,
    target: Point3,
    options: ProjectionOptions,
) -> GeomResult<SurfaceProjection> {
    if !target.is_finite() {
        return Err(GeomError::InvalidInput(
            "projection target must be finite".to_owned(),
        ));
    }
    bspline_jet(surface, 0.0, 0.0)?;
    let us = active_spans(
        &surface.u_knots,
        &surface.u_multiplicities,
        surface.u_degree,
        surface.control_points.len(),
    )?;
    let vc = surface.control_points.first().map_or(0, Vec::len);
    let vs = active_spans(
        &surface.v_knots,
        &surface.v_multiplicities,
        surface.v_degree,
        vc,
    )?;
    let bounds = (us[0].0, us[us.len() - 1].1, vs[0].0, vs[vs.len() - 1].1);
    let mut best: Option<SurfaceProjection> = None;
    let mut starts = 0_u32;
    for &(ua, ub) in &us {
        for &(va, vb) in &vs {
            for iu in 0..=options.samples_per_span() {
                for iv in 0..=options.samples_per_span() {
                    starts = starts.checked_add(1).ok_or(GeomError::BudgetExceeded {
                        resource: "projection starts",
                    })?;
                    if starts > options.max_starts() {
                        return Err(GeomError::BudgetExceeded {
                            resource: "projection starts",
                        });
                    }
                    let u = ua
                        + (ub - ua) * Scalar::from(iu) / Scalar::from(options.samples_per_span());
                    let v = va
                        + (vb - va) * Scalar::from(iv) / Scalar::from(options.samples_per_span());
                    let candidate = refine(surface, target, u, v, bounds, options)?;
                    if best
                        .as_ref()
                        .is_none_or(|current| candidate.distance < current.distance)
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    best.ok_or_else(|| GeomError::Degenerate("projection produced no candidate".to_owned()))
}

fn refine(
    surface: &BSplineSurface,
    target: Point3,
    mut u: Scalar,
    mut v: Scalar,
    bounds: (Scalar, Scalar, Scalar, Scalar),
    options: ProjectionOptions,
) -> GeomResult<SurfaceProjection> {
    let (ulo, uhi, vlo, vhi) = bounds;
    let mut iterations = 0;
    let mut status = ProjectionStatus::BudgetExhausted;
    for iteration in 0..options.max_iterations() {
        iterations = iteration + 1;
        let j = bspline_jet(surface, u, v)?;
        let r = j.point - target;
        let gu = r.dot(j.du);
        let gv = r.dot(j.dv);
        let scale = j.du.length().max(j.dv.length()).max(1.0);
        if gu.hypot(gv) <= options.tolerance().linear() * scale {
            status = ProjectionStatus::Converged;
            break;
        }
        let huu = j.du.dot(j.du) + r.dot(j.duu);
        let huv = j.du.dot(j.dv) + r.dot(j.duv);
        let hvv = j.dv.dot(j.dv) + r.dot(j.dvv);
        let det = huu * hvv - huv * huv;
        let hscale = huu.abs().max(huv.abs()).max(hvv.abs()).max(1.0);
        if !det.is_finite() || det.abs() <= Scalar::EPSILON * hscale * hscale {
            break;
        }
        let du = (-gu * hvv + huv * gv) / det;
        let dv = (huv * gu - huu * gv) / det;
        let next_u = (u + du).clamp(ulo, uhi);
        let next_v = (v + dv).clamp(vlo, vhi);
        let movement = (j.du * (next_u - u) + j.dv * (next_v - v)).length();
        u = next_u;
        v = next_v;
        if movement <= options.tolerance().linear() {
            status = ProjectionStatus::Converged;
            break;
        }
    }
    let point = bspline_jet(surface, u, v)?.point;
    let distance = point.distance(target);
    if !distance.is_finite() {
        return Err(GeomError::Degenerate(
            "projection distance is non-finite".to_owned(),
        ));
    }
    Ok(SurfaceProjection {
        u,
        v,
        point,
        distance,
        iterations,
        on_boundary: boundary(u, v, bounds),
        status,
    })
}

fn boundary(u: Scalar, v: Scalar, bounds: (Scalar, Scalar, Scalar, Scalar)) -> bool {
    u == bounds.0 || u == bounds.1 || v == bounds.2 || v == bounds.3
}
