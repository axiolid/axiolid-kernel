//! Exhaustive minimum-distance bounds for pairs of rational B-spline curves.

use crate::certified_bezier::{
    distance_between_point_intervals_upper, next_up, representative_distance, Cell,
    HomogeneousPoint,
};
use crate::certified_projection::{
    CertifiedProjectionOptions, CurveDistanceCertificate2, CurveDistanceCertificate3,
    CurvePairParameterBox, ParameterInterval,
};
use crate::certified_refinement::{piecewise_bezier_cells, RefinementBudget};
use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{BSplineCurve2, BSplineCurve3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_reference::curve::{bspline_jet2, bspline_jet3};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Certify the global minimum distance between two planar B-spline curves.
pub fn distance_curve2_certified(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    options: CertifiedProjectionOptions,
) -> GeomResult<CurveDistanceCertificate2> {
    let mut refinement_budget =
        RefinementBudget::new(options.max_nodes(), "certified curve-pair budget");
    let first_cells = piecewise_bezier_cells(
        first,
        |point| [point.x, point.y, 0.0],
        &mut refinement_budget,
    )?;
    let second_cells = piecewise_bezier_cells(
        second,
        |point| [point.x, point.y, 0.0],
        &mut refinement_budget,
    )?;
    let core = distance_core(
        first_cells,
        second_cells,
        2,
        options,
        |parameter| {
            let point = bspline_jet2(first, parameter)?.point;
            Ok([point.x, point.y, 0.0])
        },
        |parameter| {
            let point = bspline_jet2(second, parameter)?.point;
            Ok([point.x, point.y, 0.0])
        },
    )?;
    Ok(CurveDistanceCertificate2 {
        first_parameter: core.first_parameter,
        second_parameter: core.second_parameter,
        first_point: Point2::new(core.first_point[0], core.first_point[1]),
        second_point: Point2::new(core.second_point[0], core.second_point[1]),
        distance: core.distance,
        distance_lower_bound: core.lower,
        distance_upper_bound: core.upper,
        possible_minimizer_boxes: core.boxes,
        visited_nodes: core.visited_nodes,
    })
}

/// Certify the global minimum distance between two spatial B-spline curves.
pub fn distance_curve3_certified(
    first: &BSplineCurve3,
    second: &BSplineCurve3,
    options: CertifiedProjectionOptions,
) -> GeomResult<CurveDistanceCertificate3> {
    let mut refinement_budget =
        RefinementBudget::new(options.max_nodes(), "certified curve-pair budget");
    let first_cells = piecewise_bezier_cells(
        first,
        |point| [point.x, point.y, point.z],
        &mut refinement_budget,
    )?;
    let second_cells = piecewise_bezier_cells(
        second,
        |point| [point.x, point.y, point.z],
        &mut refinement_budget,
    )?;
    let core = distance_core(
        first_cells,
        second_cells,
        3,
        options,
        |parameter| {
            let point = bspline_jet3(first, parameter)?.point;
            Ok([point.x, point.y, point.z])
        },
        |parameter| {
            let point = bspline_jet3(second, parameter)?.point;
            Ok([point.x, point.y, point.z])
        },
    )?;
    Ok(CurveDistanceCertificate3 {
        first_parameter: core.first_parameter,
        second_parameter: core.second_parameter,
        first_point: Point3::new(
            core.first_point[0],
            core.first_point[1],
            core.first_point[2],
        ),
        second_point: Point3::new(
            core.second_point[0],
            core.second_point[1],
            core.second_point[2],
        ),
        distance: core.distance,
        distance_lower_bound: core.lower,
        distance_upper_bound: core.upper,
        possible_minimizer_boxes: core.boxes,
        visited_nodes: core.visited_nodes,
    })
}

#[derive(Debug)]
struct QueueCell {
    lower: Scalar,
    serial: u64,
    first: Cell,
    second: Cell,
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

#[derive(Debug)]
struct Candidate {
    first_parameter: Scalar,
    second_parameter: Scalar,
    first_point: [Scalar; 3],
    second_point: [Scalar; 3],
    distance: Scalar,
    upper: Scalar,
}

#[derive(Debug)]
struct DistanceCore {
    first_parameter: Scalar,
    second_parameter: Scalar,
    first_point: [Scalar; 3],
    second_point: [Scalar; 3],
    distance: Scalar,
    lower: Scalar,
    upper: Scalar,
    boxes: Vec<CurvePairParameterBox>,
    visited_nodes: u32,
}

fn distance_core(
    first_cells: Vec<Cell>,
    second_cells: Vec<Cell>,
    dimensions: usize,
    options: CertifiedProjectionOptions,
    evaluate_first: impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
    evaluate_second: impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<DistanceCore> {
    let initial_count =
        first_cells
            .len()
            .checked_mul(second_cells.len())
            .ok_or(GeomError::BudgetExceeded {
                resource: "certified curve-pair budget",
            })?;
    if initial_count > options.max_nodes() as usize {
        return Err(GeomError::BudgetExceeded {
            resource: "certified curve-pair budget",
        });
    }

    let mut heap = BinaryHeap::with_capacity(initial_count);
    let mut best = None;
    let mut serial = 0_u64;
    for first in first_cells {
        for second in &second_cells {
            consider_cell_samples(
                &mut best,
                &first,
                second,
                dimensions,
                &evaluate_first,
                &evaluate_second,
            )?;
            let lower = first.gap(second, dimensions)?;
            heap.push(QueueCell {
                lower,
                serial,
                first: first.clone(),
                second: second.clone(),
            });
            serial = serial.checked_add(1).ok_or(GeomError::BudgetExceeded {
                resource: "certified curve-pair budget",
            })?;
        }
    }
    let mut visited_nodes =
        u32::try_from(initial_count).map_err(|_| GeomError::BudgetExceeded {
            resource: "certified curve-pair budget",
        })?;
    let mut best = best.ok_or_else(|| GeomError::Degenerate("no pair candidate".to_owned()))?;

    loop {
        while heap.peek().is_some_and(|cell| cell.lower > best.upper) {
            heap.pop();
        }
        let global_lower = heap
            .peek()
            .map_or(best.upper, |cell| cell.lower.min(best.upper));
        if best.upper - global_lower <= options.tolerance().linear() {
            let mut boxes = heap
                .iter()
                .filter(|cell| cell.lower <= best.upper)
                .map(|cell| CurvePairParameterBox {
                    first: ParameterInterval {
                        start: cell.first.start,
                        end: cell.first.end,
                    },
                    second: ParameterInterval {
                        start: cell.second.start,
                        end: cell.second.end,
                    },
                })
                .collect::<Vec<_>>();
            boxes.push(CurvePairParameterBox {
                first: ParameterInterval {
                    start: best.first_parameter,
                    end: best.first_parameter,
                },
                second: ParameterInterval {
                    start: best.second_parameter,
                    end: best.second_parameter,
                },
            });
            boxes.sort_by(|left, right| {
                left.first
                    .start
                    .total_cmp(&right.first.start)
                    .then_with(|| left.second.start.total_cmp(&right.second.start))
            });
            return Ok(DistanceCore {
                first_parameter: best.first_parameter,
                second_parameter: best.second_parameter,
                first_point: best.first_point,
                second_point: best.second_point,
                distance: best.distance,
                lower: global_lower,
                upper: best.upper,
                boxes,
                visited_nodes,
            });
        }

        let cell = heap
            .pop()
            .ok_or_else(|| GeomError::Degenerate("curve-pair queue exhausted".to_owned()))?;
        let first_span = (cell.first.end - cell.first.start).abs();
        let second_span = (cell.second.end - cell.second.start).abs();
        let split_first = cell.first.depth < options.max_depth()
            && (cell.second.depth >= options.max_depth() || first_span >= second_span);
        if !split_first && cell.second.depth >= options.max_depth() {
            return Err(GeomError::BudgetExceeded {
                resource: "certified curve-pair budget",
            });
        }
        let requested = visited_nodes
            .checked_add(2)
            .ok_or(GeomError::BudgetExceeded {
                resource: "certified curve-pair budget",
            })?;
        if requested > options.max_nodes() {
            return Err(GeomError::BudgetExceeded {
                resource: "certified curve-pair budget",
            });
        }

        let children = if split_first {
            let (left, right) = cell.first.split()?;
            [(left, cell.second.clone()), (right, cell.second)]
        } else {
            let (left, right) = cell.second.split()?;
            [(cell.first.clone(), left), (cell.first, right)]
        };
        visited_nodes = requested;
        for (first, second) in children {
            consider_midpoint_pair(
                &mut best,
                &first,
                &second,
                dimensions,
                &evaluate_first,
                &evaluate_second,
            )?;
            let lower = first.gap(&second, dimensions)?;
            if lower <= best.upper {
                heap.push(QueueCell {
                    lower,
                    serial,
                    first,
                    second,
                });
                serial = serial.checked_add(1).ok_or(GeomError::BudgetExceeded {
                    resource: "certified curve-pair budget",
                })?;
            }
        }
    }
}

fn consider_cell_samples(
    best: &mut impl CandidateSlot,
    first: &Cell,
    second: &Cell,
    dimensions: usize,
    evaluate_first: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
    evaluate_second: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<()> {
    let first_samples = cell_samples(first)?;
    let second_samples = cell_samples(second)?;
    for (first_parameter, first_enclosure) in &first_samples {
        for (second_parameter, second_enclosure) in &second_samples {
            consider_candidate(
                best,
                *first_parameter,
                *second_parameter,
                first_enclosure,
                second_enclosure,
                dimensions,
                evaluate_first,
                evaluate_second,
            )?;
        }
    }
    Ok(())
}

fn consider_midpoint_pair(
    best: &mut Candidate,
    first: &Cell,
    second: &Cell,
    dimensions: usize,
    evaluate_first: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
    evaluate_second: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<()> {
    let first_parameter = first.start * 0.5 + first.end * 0.5;
    let second_parameter = second.start * 0.5 + second.end * 0.5;
    let first_enclosure = first.midpoint_point()?;
    let second_enclosure = second.midpoint_point()?;
    consider_candidate(
        best,
        first_parameter,
        second_parameter,
        &first_enclosure,
        &second_enclosure,
        dimensions,
        evaluate_first,
        evaluate_second,
    )
}

fn cell_samples(cell: &Cell) -> GeomResult<Vec<(Scalar, HomogeneousPoint)>> {
    Ok(vec![
        (cell.start, cell.controls[0].clone()),
        (cell.start * 0.5 + cell.end * 0.5, cell.midpoint_point()?),
        (cell.end, cell.controls[cell.controls.len() - 1].clone()),
    ])
}

#[allow(clippy::too_many_arguments)]
fn consider_candidate(
    best: &mut impl CandidateSlot,
    first_parameter: Scalar,
    second_parameter: Scalar,
    first_enclosure: &HomogeneousPoint,
    second_enclosure: &HomogeneousPoint,
    dimensions: usize,
    evaluate_first: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
    evaluate_second: &impl Fn(Scalar) -> GeomResult<[Scalar; 3]>,
) -> GeomResult<()> {
    let first_point = evaluate_first(first_parameter)?;
    let second_point = evaluate_second(second_parameter)?;
    let distance = representative_distance(first_point, second_point, dimensions)?;
    let upper = distance_between_point_intervals_upper(
        first_enclosure.euclidean()?,
        second_enclosure.euclidean()?,
        dimensions,
    )?
    .max(next_up(distance));
    let candidate = Candidate {
        first_parameter,
        second_parameter,
        first_point,
        second_point,
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
        match self {
            Some(current)
                if candidate.upper > current.upper
                    || (candidate.upper == current.upper
                        && (candidate.first_parameter, candidate.second_parameter)
                            >= (current.first_parameter, current.second_parameter)) => {}
            slot => *slot = Some(candidate),
        }
    }
}

impl CandidateSlot for Candidate {
    fn consider(&mut self, candidate: Candidate) {
        if candidate.upper < self.upper
            || (candidate.upper == self.upper
                && (candidate.first_parameter, candidate.second_parameter)
                    < (self.first_parameter, self.second_parameter))
        {
            *self = candidate;
        }
    }
}
