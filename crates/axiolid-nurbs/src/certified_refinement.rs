//! Interval-aware clamped NURBS refinement into conservative rational Bézier cells.

use crate::certified_bezier::{Cell, HomogeneousPoint, Interval};
use axiolid_core::Scalar;
use axiolid_curve::BSplineCurve;
use axiolid_kernel::{GeomError, GeomResult};

pub(crate) struct RefinementBudget {
    remaining: u128,
    resource: &'static str,
}

impl RefinementBudget {
    pub(crate) fn new(limit: u32, resource: &'static str) -> Self {
        Self {
            remaining: u128::from(limit),
            resource,
        }
    }

    pub(crate) fn charge(&mut self, work: Option<u128>) -> GeomResult<()> {
        let Some(work) = work else {
            return Err(GeomError::BudgetExceeded {
                resource: self.resource,
            });
        };
        if work > self.remaining {
            return Err(GeomError::BudgetExceeded {
                resource: self.resource,
            });
        }
        self.remaining -= work;
        Ok(())
    }
}

fn refinement_work<P>(curve: &BSplineCurve<P>) -> Option<u128> {
    let controls = curve.control_points.len() as u128;
    let degree = u128::from(curve.degree);
    let expanded_knots = controls.checked_add(degree)?.checked_add(1)?;
    let insertions = curve.multiplicities[1..curve.multiplicities.len() - 1]
        .iter()
        .try_fold(0_u128, |total, &present| {
            let missing = degree.checked_sub(u128::from(present))?;
            total.checked_add(missing)
        })?;
    let triangular = insertions
        .checked_mul(insertions.checked_add(1)?)?
        .checked_div(2)?;
    let inserted_controls = insertions.checked_mul(controls)?.checked_add(triangular)?;
    let inserted_knots = insertions
        .checked_mul(expanded_knots)?
        .checked_add(triangular)?;
    let insertion_work = inserted_controls
        .checked_mul(2)?
        .checked_add(inserted_knots)?;
    let segments = curve.knots.len().checked_sub(1)? as u128;
    let emitted_controls = segments.checked_mul(degree.checked_add(1)?)?;

    controls
        .checked_add(expanded_knots)?
        .checked_add(insertion_work)?
        .checked_add(emitted_controls)?
        .checked_add(segments)
}

pub(crate) fn piecewise_bezier_cells<P>(
    curve: &BSplineCurve<P>,
    coordinates: impl Fn(&P) -> [Scalar; 3],
    budget: &mut RefinementBudget,
) -> GeomResult<Vec<Cell>> {
    let degree = usize::from(curve.degree);
    validate_clamped_axis(curve, degree)?;
    budget.charge(refinement_work(curve))?;

    let mut controls = homogeneous_controls(curve, coordinates)?;
    let mut expanded = expand_knots(curve)?;
    for distinct in 1..curve.knots.len() - 1 {
        let knot = curve.knots[distinct];
        let present = usize::try_from(curve.multiplicities[distinct]).map_err(|_| {
            GeomError::InvalidInput("knot multiplicity does not fit usize".to_owned())
        })?;
        for _ in present..degree {
            (controls, expanded) = insert_homogeneous_once(controls, expanded, degree, knot)?;
        }
    }

    let segment_count = curve.knots.len() - 1;
    let expected_controls = segment_count
        .checked_mul(degree)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| GeomError::InvalidInput("Bézier control count overflows".to_owned()))?;
    if controls.len() != expected_controls {
        return Err(GeomError::Degenerate(format!(
            "refined Bézier curve has {} controls, expected {expected_controls}",
            controls.len()
        )));
    }

    let mut cells = Vec::with_capacity(segment_count);
    for segment in 0..segment_count {
        let first = segment * degree;
        cells.push(Cell {
            controls: controls[first..=first + degree].to_vec(),
            start: curve.knots[segment],
            end: curve.knots[segment + 1],
            depth: 0,
        });
    }
    Ok(cells)
}

fn validate_clamped_axis<P>(curve: &BSplineCurve<P>, degree: usize) -> GeomResult<()> {
    if degree == 0 || curve.knots.len() < 2 || curve.knots.len() != curve.multiplicities.len() {
        return Err(GeomError::InvalidInput(
            "certified projection requires a valid clamped NURBS axis".to_owned(),
        ));
    }
    let end_multiplicity = u32::from(curve.degree) + 1;
    if curve.multiplicities.first() != Some(&end_multiplicity)
        || curve.multiplicities.last() != Some(&end_multiplicity)
        || curve.multiplicities[1..curve.multiplicities.len() - 1]
            .iter()
            .any(|&multiplicity| multiplicity == 0 || multiplicity > u32::from(curve.degree))
        || curve.knots.iter().any(|knot| !knot.is_finite())
        || curve.knots.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(GeomError::InvalidInput(
            "certified projection requires finite increasing clamped knots with continuous internal spans"
                .to_owned(),
        ));
    }
    if curve.control_points.len() <= degree {
        return Err(GeomError::InvalidInput(
            "NURBS degree requires more control points".to_owned(),
        ));
    }
    let expected = curve
        .control_points
        .len()
        .checked_add(degree)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| GeomError::InvalidInput("NURBS size overflows".to_owned()))?;
    let mut total = 0usize;
    for &multiplicity in &curve.multiplicities {
        total = total
            .checked_add(usize::try_from(multiplicity).map_err(|_| {
                GeomError::InvalidInput("knot multiplicity does not fit usize".to_owned())
            })?)
            .ok_or_else(|| GeomError::InvalidInput("knot multiplicity sum overflows".to_owned()))?;
        if total > expected {
            return Err(GeomError::InvalidInput(
                "knot multiplicity exceeds the expected total".to_owned(),
            ));
        }
    }
    if total != expected {
        return Err(GeomError::InvalidInput(format!(
            "knot vector has {total} entries, expected {expected}"
        )));
    }
    Ok(())
}

fn homogeneous_controls<P>(
    curve: &BSplineCurve<P>,
    coordinates: impl Fn(&P) -> [Scalar; 3],
) -> GeomResult<Vec<HomogeneousPoint>> {
    if let Some(weights) = &curve.weights {
        if weights.len() != curve.control_points.len()
            || weights
                .iter()
                .any(|weight| !weight.is_finite() || *weight <= 0.0)
        {
            return Err(GeomError::InvalidInput(
                "certified projection requires one positive finite weight per control".to_owned(),
            ));
        }
    }
    let mut controls = Vec::with_capacity(curve.control_points.len());
    for (index, point) in curve.control_points.iter().enumerate() {
        let coordinate = coordinates(point);
        if coordinate.iter().any(|value| !value.is_finite()) {
            return Err(GeomError::InvalidInput(
                "certified projection control points must be finite".to_owned(),
            ));
        }
        let weight = curve.weights.as_ref().map_or(1.0, |weights| weights[index]);
        let numerator = if curve.weights.is_some() {
            [
                Interval::product(coordinate[0], weight)?,
                Interval::product(coordinate[1], weight)?,
                Interval::product(coordinate[2], weight)?,
            ]
        } else {
            [
                Interval::exact(coordinate[0])?,
                Interval::exact(coordinate[1])?,
                Interval::exact(coordinate[2])?,
            ]
        };
        controls.push(HomogeneousPoint {
            numerator,
            weight: Interval::exact(weight)?,
        });
    }
    Ok(controls)
}

fn expand_knots<P>(curve: &BSplineCurve<P>) -> GeomResult<Vec<Scalar>> {
    let expected = curve
        .control_points
        .len()
        .checked_add(usize::from(curve.degree))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| GeomError::InvalidInput("NURBS size overflows".to_owned()))?;
    let mut expanded = Vec::with_capacity(expected);
    for (&knot, &multiplicity) in curve.knots.iter().zip(&curve.multiplicities) {
        expanded.extend(core::iter::repeat_n(knot, multiplicity as usize));
    }
    Ok(expanded)
}

pub(crate) fn insert_homogeneous_once(
    controls: Vec<HomogeneousPoint>,
    knots: Vec<Scalar>,
    degree: usize,
    knot: Scalar,
) -> GeomResult<(Vec<HomogeneousPoint>, Vec<Scalar>)> {
    let last_control = controls
        .len()
        .checked_sub(1)
        .ok_or_else(|| GeomError::InvalidInput("empty NURBS control polygon".to_owned()))?;
    let span = (degree..=last_control)
        .find(|&index| knot >= knots[index] && knot < knots[index + 1])
        .or_else(|| (knot == knots[last_control + 1]).then_some(last_control))
        .ok_or_else(|| {
            GeomError::InvalidInput("inserted knot is outside the active domain".to_owned())
        })?;
    let multiplicity = knots.iter().filter(|&&value| value == knot).count();
    if multiplicity >= degree {
        return Err(GeomError::InvalidInput(
            "internal knot is already fully refined".to_owned(),
        ));
    }

    let refined_len = controls
        .len()
        .checked_add(1)
        .ok_or(GeomError::BudgetExceeded {
            resource: "certified NURBS refinement allocation",
        })?;
    let mut refined: Vec<Option<HomogeneousPoint>> = Vec::new();
    refined
        .try_reserve_exact(refined_len)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "certified NURBS refinement allocation",
        })?;
    refined.resize_with(refined_len, || None);
    for index in 0..=span - degree {
        refined[index] = Some(controls[index].clone());
    }
    for index in span - multiplicity..=last_control {
        refined[index + 1] = Some(controls[index].clone());
    }
    for index in span - degree + 1..=span - multiplicity {
        let numerator = Interval::exact(knot)?.subtract(Interval::exact(knots[index])?)?;
        let denominator =
            Interval::exact(knots[index + degree])?.subtract(Interval::exact(knots[index])?)?;
        let alpha = numerator.divide(denominator)?;
        refined[index] = Some(HomogeneousPoint::blend(
            &controls[index - 1],
            &controls[index],
            alpha,
        )?);
    }
    let mut completed = Vec::new();
    completed
        .try_reserve_exact(refined_len)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "certified NURBS refinement allocation",
        })?;
    for (index, control) in refined.into_iter().enumerate() {
        completed.push(control.ok_or_else(|| {
            GeomError::Degenerate(format!("knot insertion left refined control {index} unset"))
        })?);
    }
    let refined_knot_len = knots
        .len()
        .checked_add(1)
        .ok_or(GeomError::BudgetExceeded {
            resource: "certified NURBS refinement allocation",
        })?;
    let mut refined_knots = Vec::new();
    refined_knots
        .try_reserve_exact(refined_knot_len)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "certified NURBS refinement allocation",
        })?;
    refined_knots.extend_from_slice(&knots[..=span]);
    refined_knots.push(knot);
    refined_knots.extend_from_slice(&knots[span + 1..]);
    Ok((completed, refined_knots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiolid_core::Point2;
    use axiolid_curve::KnotSpec;
    use axiolid_scalar::curve::bspline_jet2;

    #[test]
    fn refinement_budget_rejects_amplification_before_control_conversion() {
        let degree = 64_u16;
        let curve = BSplineCurve {
            degree,
            control_points: vec![Point2::ZERO; usize::from(degree) + 2],
            knots: vec![0.0, 0.5, 1.0],
            multiplicities: vec![u32::from(degree) + 1, 1, u32::from(degree) + 1],
            weights: None,
            closed: false,
            self_intersect: Some(false),
            knot_spec: KnotSpec::Unspecified,
        };
        let conversions = std::cell::Cell::new(0_usize);
        let mut budget = RefinementBudget::new(32, "test refinement nodes");

        let error = match piecewise_bezier_cells(
            &curve,
            |point| {
                conversions.set(conversions.get() + 1);
                [point.x, point.y, 0.0]
            },
            &mut budget,
        ) {
            Err(error) => error,
            Ok(_) => panic!(
                "refinement amplification was accepted after {} conversions",
                conversions.get()
            ),
        };

        assert!(matches!(
            error,
            GeomError::BudgetExceeded {
                resource: "test refinement nodes"
            }
        ));
        assert_eq!(conversions.get(), 0, "rejection must precede allocation");
    }

    #[test]
    fn refinement_budget_is_shared_across_pair_inputs() {
        let line = BSplineCurve {
            degree: 1,
            control_points: vec![Point2::ZERO, Point2::X],
            knots: vec![0.0, 1.0],
            multiplicities: vec![2, 2],
            weights: None,
            closed: false,
            self_intersect: Some(false),
            knot_spec: KnotSpec::PiecewiseBezier,
        };
        let mut budget = RefinementBudget::new(17, "test pair refinement nodes");
        piecewise_bezier_cells(&line, |point| [point.x, point.y, 0.0], &mut budget)
            .expect("one line costs nine refinement units");

        let conversions = std::cell::Cell::new(0_usize);
        let error = piecewise_bezier_cells(
            &line,
            |point| {
                conversions.set(conversions.get() + 1);
                [point.x, point.y, 0.0]
            },
            &mut budget,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            GeomError::BudgetExceeded {
                resource: "test pair refinement nodes"
            }
        ));
        assert_eq!(conversions.get(), 0);
    }

    #[test]
    fn polynomial_derivative_enclosure_uses_native_parameter() {
        let curve = BSplineCurve {
            degree: 1,
            control_points: vec![Point2::ZERO, Point2::new(6.0, 3.0)],
            knots: vec![2.0, 5.0],
            multiplicities: vec![2, 2],
            weights: None,
            closed: false,
            self_intersect: Some(false),
            knot_spec: KnotSpec::PiecewiseBezier,
        };
        let mut budget = RefinementBudget::new(100, "test refinement nodes");
        let cells = piecewise_bezier_cells(&curve, |point| [point.x, point.y, 0.0], &mut budget)
            .expect("line refines");
        let derivative = cells[0]
            .derivative_intervals()
            .expect("derivative encloses");
        assert!(derivative[0].contains(2.0));
        assert!(derivative[1].contains(1.0));
    }

    #[test]
    fn refined_seam_encloses_the_original_smooth_rational_curve() {
        let curve = BSplineCurve {
            degree: 2,
            control_points: vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(2.0, -1.0),
                Point2::new(3.0, 0.0),
            ],
            knots: vec![0.0, 0.25, 1.0],
            multiplicities: vec![3, 1, 3],
            weights: Some(vec![1.0, 0.75, 1.25, 1.0]),
            closed: false,
            self_intersect: Some(false),
            knot_spec: KnotSpec::Unspecified,
        };
        let mut budget = RefinementBudget::new(10_000, "test refinement nodes");
        let cells = piecewise_bezier_cells(&curve, |point| [point.x, point.y, 0.0], &mut budget)
            .expect("smooth curve refines");
        assert_eq!(cells.len(), 2);

        for cell in &cells {
            let parameter = cell.start * 0.5 + cell.end * 0.5;
            let refined = cell
                .midpoint_point()
                .expect("refined span evaluates")
                .euclidean()
                .expect("positive midpoint weight");
            let jet = bspline_jet2(&curve, parameter).expect("original evaluates inside the span");
            let original = jet.point;
            assert!(refined[0].contains(original.x));
            assert!(refined[1].contains(original.y));

            let derivative = cell
                .derivative_intervals()
                .expect("rational derivative enclosure is valid");
            for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let parameter = cell.start * (1.0 - fraction) + cell.end * fraction;
                let sample = bspline_jet2(&curve, parameter).expect("sample evaluates");
                assert!(derivative[0].contains(sample.first.x));
                assert!(derivative[1].contains(sample.first.y));
            }
        }

        let seam = cells[0]
            .controls
            .last()
            .expect("left segment has an endpoint")
            .euclidean()
            .expect("positive seam weight");
        let original = bspline_jet2(&curve, 0.25)
            .expect("original evaluates at the seam")
            .point;
        assert!(seam[0].contains(original.x));
        assert!(seam[1].contains(original.y));
    }
}
