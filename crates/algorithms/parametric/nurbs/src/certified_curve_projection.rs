//! Exhaustive closest-point bounds for piecewise rational Bézier curves.

use crate::certified_bezier::{
    distance_to_point_interval_upper, next_up, representative_distance, Cell, HomogeneousPoint,
};
use crate::certified_projection::{
    CertifiedProjectionOptions, CurveProjectionCertificate2, CurveProjectionCertificate3,
    ParameterInterval,
};
use crate::certified_refinement::{piecewise_bezier_cells, RefinementBudget};
use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{BSplineCurve2, BSplineCurve3};
use axiolid_evaluate::curve::{bspline_jet2, bspline_jet3};
use core::cmp::Ordering;
use std::collections::BinaryHeap;

/// Globally bound the closest point on a clamped planar B-spline curve.
///
/// Positive rational weights make every refined segment lie in the convex hull
/// of its Euclidean control points. Interval-aware homogeneous knot insertion
/// and outward-rounded subdivision tighten those hulls until the global
/// distance gap meets the requested tolerance.
pub fn project_curve2_certified(
    curve: &BSplineCurve2,
    target: Point2,
    options: CertifiedProjectionOptions,
) -> GeomResult<CurveProjectionCertificate2> {
    let mut refinement_budget =
        RefinementBudget::new(options.max_nodes(), "certified projection nodes");
    let roots = piecewise_bezier_cells(
        curve,
        |point| [point.x, point.y, 0.0],
        &mut refinement_budget,
    )?;
    let result = project(roots, [target.x, target.y, 0.0], 2, options, |parameter| {
        let point = bspline_jet2(curve, parameter)?.point;
        Ok([point.x, point.y, 0.0])
    })?;
    Ok(CurveProjectionCertificate2 {
        parameter: result.parameter,
        point: Point2::new(result.point[0], result.point[1]),
        distance: result.distance,
        distance_lower_bound: result.lower,
        distance_upper_bound: result.upper,
        possible_minimizer_intervals: result.intervals,
        visited_nodes: result.nodes,
    })
}

/// Globally bound the closest point on a clamped spatial B-spline curve.
///
/// See [`project_curve2_certified`] for the certification and current input
/// scope. A budget failure never returns a partial certificate.
pub fn project_curve3_certified(
    curve: &BSplineCurve3,
    target: Point3,
    options: CertifiedProjectionOptions,
) -> GeomResult<CurveProjectionCertificate3> {
    let mut refinement_budget =
        RefinementBudget::new(options.max_nodes(), "certified projection nodes");
    let roots = piecewise_bezier_cells(
        curve,
        |point| [point.x, point.y, point.z],
        &mut refinement_budget,
    )?;
    let result = project(roots, target.to_array(), 3, options, |parameter| {
        let point = bspline_jet3(curve, parameter)?.point;
        Ok(point.to_array())
    })?;
    Ok(CurveProjectionCertificate3 {
        parameter: result.parameter,
        point: Point3::from_array(result.point),
        distance: result.distance,
        distance_lower_bound: result.lower,
        distance_upper_bound: result.upper,
        possible_minimizer_intervals: result.intervals,
        visited_nodes: result.nodes,
    })
}

#[derive(Debug)]
struct QueueCell {
    lower: Scalar,
    serial: u64,
    cell: Cell,
}

impl PartialEq for QueueCell {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial
    }
}
impl Eq for QueueCell {}
impl PartialOrd for QueueCell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueueCell {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .lower
            .total_cmp(&self.lower)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    parameter: Scalar,
    point: [Scalar; 3],
    distance: Scalar,
    upper: Scalar,
}

struct ProjectionCore {
    parameter: Scalar,
    point: [Scalar; 3],
    distance: Scalar,
    lower: Scalar,
    upper: Scalar,
    intervals: Vec<ParameterInterval>,
    nodes: u32,
}

fn project(
    roots: Vec<Cell>,
    target: [Scalar; 3],
    dimensions: usize,
    options: CertifiedProjectionOptions,
    evaluate: impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<ProjectionCore> {
    if target[..dimensions].iter().any(|value| !value.is_finite()) {
        return Err(GeomError::InvalidInput(
            "projection target must be finite".to_owned(),
        ));
    }
    let mut nodes = u32::try_from(roots.len()).map_err(|_| GeomError::BudgetExceeded {
        resource: "certified projection nodes",
    })?;
    if nodes > options.max_nodes() {
        return Err(GeomError::BudgetExceeded {
            resource: "certified projection nodes",
        });
    }

    let mut best: Option<Candidate> = None;
    let mut heap = BinaryHeap::new();
    let mut serial = 0_u64;
    for cell in roots {
        consider_parameter(
            &mut best,
            cell.start,
            &cell.controls[0],
            target,
            dimensions,
            &evaluate,
        )?;
        consider_parameter(
            &mut best,
            cell.end,
            &cell.controls[cell.controls.len() - 1],
            target,
            dimensions,
            &evaluate,
        )?;
        let midpoint = cell.start * 0.5 + cell.end * 0.5;
        consider_parameter(
            &mut best,
            midpoint,
            &cell.midpoint_point()?,
            target,
            dimensions,
            &evaluate,
        )?;
        let lower = cell.lower_bound(target, dimensions)?;
        heap.push(QueueCell {
            lower,
            serial,
            cell,
        });
        serial += 1;
    }
    let mut best = best.ok_or_else(|| {
        GeomError::InvalidInput("certified projection has no curve segments".to_owned())
    })?;

    loop {
        while heap.peek().is_some_and(|entry| entry.lower > best.upper) {
            heap.pop();
        }
        let global_lower = heap
            .peek()
            .map_or(best.upper, |entry| entry.lower.min(best.upper));
        if best.upper - global_lower <= options.tolerance().linear() {
            let mut intervals: Vec<_> = heap
                .iter()
                .filter(|entry| entry.lower <= best.upper)
                .map(|entry| ParameterInterval {
                    start: entry.cell.start,
                    end: entry.cell.end,
                })
                .collect();
            intervals.push(ParameterInterval {
                start: best.parameter,
                end: best.parameter,
            });
            intervals.sort_by(|left, right| left.start.total_cmp(&right.start));
            return Ok(ProjectionCore {
                parameter: best.parameter,
                point: best.point,
                distance: best.distance,
                lower: global_lower,
                upper: best.upper,
                intervals,
                nodes,
            });
        }

        let current = heap.pop().ok_or_else(|| {
            GeomError::Degenerate("certified projection queue became empty".to_owned())
        })?;
        if current.cell.depth >= options.max_depth() {
            return Err(GeomError::BudgetExceeded {
                resource: "certified projection depth",
            });
        }
        let next_nodes = nodes.checked_add(2).ok_or(GeomError::BudgetExceeded {
            resource: "certified projection nodes",
        })?;
        if next_nodes > options.max_nodes() {
            return Err(GeomError::BudgetExceeded {
                resource: "certified projection nodes",
            });
        }
        nodes = next_nodes;
        let (left, right) = current.cell.split()?;
        for child in [left, right] {
            let midpoint = child.start * 0.5 + child.end * 0.5;
            consider_parameter(
                &mut best,
                midpoint,
                &child.midpoint_point()?,
                target,
                dimensions,
                &evaluate,
            )?;
            let lower = child.lower_bound(target, dimensions)?;
            if lower <= best.upper {
                heap.push(QueueCell {
                    lower,
                    serial,
                    cell: child,
                });
                serial = serial.checked_add(1).ok_or(GeomError::BudgetExceeded {
                    resource: "certified projection serials",
                })?;
            }
        }
    }
}

fn consider_parameter(
    best: &mut impl CandidateSlot,
    parameter: Scalar,
    enclosure: &HomogeneousPoint,
    target: [Scalar; 3],
    dimensions: usize,
    evaluate: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<()> {
    let point = evaluate(parameter)?;
    if point[..dimensions].iter().any(|value| !value.is_finite()) {
        return Err(GeomError::Degenerate(
            "scalar projection evaluation is non-finite".to_owned(),
        ));
    }
    let distance = representative_distance(point, target, dimensions)?;
    let upper = distance_to_point_interval_upper(target, enclosure.euclidean()?, dimensions)?
        .max(next_up(distance));
    let candidate = Candidate {
        parameter,
        point,
        distance,
        upper,
    };
    best.consider(candidate);
    Ok(())
}

trait CandidateSlot {
    fn consider(&mut self, candidate: Candidate);
}

impl CandidateSlot for Option<Candidate> {
    fn consider(&mut self, candidate: Candidate) {
        if self.as_ref().is_none_or(|current| {
            candidate.upper < current.upper
                || (candidate.upper == current.upper && candidate.parameter < current.parameter)
        }) {
            *self = Some(candidate);
        }
    }
}

impl CandidateSlot for Candidate {
    fn consider(&mut self, candidate: Candidate) {
        if candidate.upper < self.upper
            || (candidate.upper == self.upper && candidate.parameter < self.parameter)
        {
            *self = candidate;
        }
    }
}
