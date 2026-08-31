//! Certified planar curve/curve relationship classification.

use crate::certified_bezier::{distance_between_point_intervals_upper, Interval};
use crate::certified_projection::{
    CertifiedProjectionOptions, CurvePairParameterBox, ParameterInterval,
};
use crate::certified_refinement::{piecewise_bezier_cells, RefinementBudget};
use axiolid_core::{Point2, Scalar};
use axiolid_curve::BSplineCurve2;
use axiolid_kernel::{GeomResult, Sign};
use axiolid_scalar::{curve::bspline_jet2, orient2d};

/// One isolated transverse planar intersection.
#[derive(Debug, Clone, PartialEq)]
pub struct TransverseCurveIntersection2 {
    /// Certified parameter interval on the first curve.
    pub first_parameter: ParameterInterval,
    /// Certified parameter interval on the second curve.
    pub second_parameter: ParameterInterval,
    /// Scalar-oracle representative of the common point.
    pub point: Point2,
    /// Residual of the representative evaluations, rounded upward.
    pub residual_upper_bound: Scalar,
    /// Conservative absolute lower bound on the root Jacobian determinant.
    pub jacobian_determinant_lower_bound: Scalar,
}

/// Singular or not-yet-isolated curve relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveIntersectionDegeneracy {
    /// A structurally proven endpoint tangency.
    Tangency,
    /// A structurally proven positive-dimensional overlap.
    Overlap,
    /// Candidate boxes remain but no supported proof classified them.
    Unresolved,
}

/// Exhaustive planar curve/curve classification under the implemented proof paths.
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedCurveIntersection2 {
    /// Every product-domain cell was excluded or represented by one transverse root.
    Complete {
        /// Isolated transverse roots. Empty means certified disjointness.
        intersections: Vec<TransverseCurveIntersection2>,
        /// Generated root cells inspected by the classifier.
        visited_nodes: u32,
    },
    /// A singular or currently unsupported candidate was not called transverse.
    Degenerate {
        /// Proven structural class, or `Unresolved` when no proof completed.
        classification: CurveIntersectionDegeneracy,
        /// Product-domain boxes containing the singular/unresolved relationship.
        candidate_boxes: Vec<CurvePairParameterBox>,
        /// Generated root cells inspected by the classifier.
        visited_nodes: u32,
    },
}

/// Classify intersections of two clamped planar B-spline curves.
///
/// The current complete proof path covers single-span polynomial line segments.
/// It also proves identical-curve overlap and a polynomial quadratic endpoint
/// tangency pattern from exact orientation signs and the Bézier convex hull.
/// Other nonlinear candidates return [`CurveIntersectionDegeneracy::Unresolved`];
/// they are never reported as certified transverse roots.
pub fn intersect_curve2_certified(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    options: CertifiedProjectionOptions,
) -> GeomResult<CertifiedCurveIntersection2> {
    let mut budget =
        RefinementBudget::new(options.max_nodes(), "certified curve intersection budget");
    let first_cells = piecewise_bezier_cells(first, |p| [p.x, p.y, 0.0], &mut budget)?;
    let second_cells = piecewise_bezier_cells(second, |p| [p.x, p.y, 0.0], &mut budget)?;
    let count = first_cells.len().checked_mul(second_cells.len());
    let visited_nodes = count
        .and_then(|value| u32::try_from(value).ok())
        .filter(|&value| value <= options.max_nodes())
        .ok_or(axiolid_kernel::GeomError::BudgetExceeded {
            resource: "certified curve intersection budget",
        })?;
    let boxes = product_boxes(&first_cells, &second_cells);

    if first == second {
        return Ok(CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Overlap,
            candidate_boxes: boxes,
            visited_nodes,
        });
    }
    if structural_start_tangency(first, second) || structural_start_tangency(second, first) {
        return Ok(CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Tangency,
            candidate_boxes: boxes,
            visited_nodes,
        });
    }
    if linear_polynomial(first) && linear_polynomial(second) {
        return classify_lines(first, second, visited_nodes, boxes);
    }

    classify_nonlinear(
        first,
        second,
        first_cells,
        second_cells,
        visited_nodes,
        options,
    )
}

fn classify_nonlinear(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    first_cells: Vec<crate::certified_bezier::Cell>,
    second_cells: Vec<crate::certified_bezier::Cell>,
    mut visited_nodes: u32,
    options: CertifiedProjectionOptions,
) -> GeomResult<CertifiedCurveIntersection2> {
    let mut pending = first_cells
        .into_iter()
        .flat_map(|first_cell| {
            second_cells
                .iter()
                .cloned()
                .map(move |second_cell| (first_cell.clone(), second_cell, 0_u16))
        })
        .collect::<Vec<_>>();
    let mut intersections = Vec::new();
    let mut unresolved = Vec::new();

    while let Some((first_cell, second_cell, depth)) = pending.pop() {
        if residual_excludes_zero(&first_cell, &second_cell)? {
            continue;
        }
        if let Some(root) = krawczyk_root(first, second, &first_cell, &second_cell)? {
            intersections.push(root);
            continue;
        }
        if depth >= options.max_depth() {
            unresolved.push(pair_box(&first_cell, &second_cell));
            continue;
        }
        visited_nodes = visited_nodes
            .checked_add(2)
            .filter(|&count| count <= options.max_nodes())
            .ok_or(axiolid_kernel::GeomError::BudgetExceeded {
                resource: "certified curve intersection budget",
            })?;
        if first_cell.end - first_cell.start >= second_cell.end - second_cell.start {
            let (left, right) = first_cell.split()?;
            pending.push((left, second_cell.clone(), depth + 1));
            pending.push((right, second_cell, depth + 1));
        } else {
            let (left, right) = second_cell.split()?;
            pending.push((first_cell.clone(), left, depth + 1));
            pending.push((first_cell, right, depth + 1));
        }
    }

    if unresolved.is_empty() {
        Ok(CertifiedCurveIntersection2::Complete {
            intersections,
            visited_nodes,
        })
    } else {
        Ok(CertifiedCurveIntersection2::Degenerate {
            classification: CurveIntersectionDegeneracy::Unresolved,
            candidate_boxes: unresolved,
            visited_nodes,
        })
    }
}

fn residual_excludes_zero(
    first: &crate::certified_bezier::Cell,
    second: &crate::certified_bezier::Cell,
) -> GeomResult<bool> {
    let first = first.coordinate_intervals()?;
    let second = second.coordinate_intervals()?;
    Ok(!first[0].subtract(second[0])?.contains_zero()
        || !first[1].subtract(second[1])?.contains_zero())
}

fn pair_box(
    first: &crate::certified_bezier::Cell,
    second: &crate::certified_bezier::Cell,
) -> CurvePairParameterBox {
    CurvePairParameterBox {
        first: ParameterInterval {
            start: first.start,
            end: first.end,
        },
        second: ParameterInterval {
            start: second.start,
            end: second.end,
        },
    }
}

fn krawczyk_root(
    first_curve: &BSplineCurve2,
    second_curve: &BSplineCurve2,
    first: &crate::certified_bezier::Cell,
    second: &crate::certified_bezier::Cell,
) -> GeomResult<Option<TransverseCurveIntersection2>> {
    let first_mid = first.start * 0.5 + first.end * 0.5;
    let second_mid = second.start * 0.5 + second.end * 0.5;
    let first_jet = bspline_jet2(first_curve, first_mid)?;
    let second_jet = bspline_jet2(second_curve, second_mid)?;
    let j00 = first_jet.first.x;
    let j01 = -second_jet.first.x;
    let j10 = first_jet.first.y;
    let j11 = -second_jet.first.y;
    let determinant = j00 * j11 - j01 * j10;
    if determinant == 0.0 || !determinant.is_finite() {
        return Ok(None);
    }
    let inverse = [
        [j11 / determinant, -j01 / determinant],
        [-j10 / determinant, j00 / determinant],
    ];
    if inverse.iter().flatten().any(|value| !value.is_finite()) {
        return Ok(None);
    }

    let first_point = first.midpoint_point()?.euclidean()?;
    let second_point = second.midpoint_point()?.euclidean()?;
    let residual = [
        first_point[0].subtract(second_point[0])?,
        first_point[1].subtract(second_point[1])?,
    ];
    let first_derivative = first.derivative_intervals()?;
    let second_derivative = second.derivative_intervals()?;
    let minus_one = Interval::exact(-1.0)?;
    let jacobian = [
        [
            first_derivative[0],
            second_derivative[0].multiply(minus_one)?,
        ],
        [
            first_derivative[1],
            second_derivative[1].multiply(minus_one)?,
        ],
    ];

    let corrected = [
        Interval::exact(first_mid)?.subtract(linear_combination(
            inverse[0][0],
            residual[0],
            inverse[0][1],
            residual[1],
        )?)?,
        Interval::exact(second_mid)?.subtract(linear_combination(
            inverse[1][0],
            residual[0],
            inverse[1][1],
            residual[1],
        )?)?,
    ];
    let zero = Interval::exact(0.0)?;
    let one = Interval::exact(1.0)?;
    let matrix = [
        [
            one.subtract(linear_combination(
                inverse[0][0],
                jacobian[0][0],
                inverse[0][1],
                jacobian[1][0],
            )?)?,
            zero.subtract(linear_combination(
                inverse[0][0],
                jacobian[0][1],
                inverse[0][1],
                jacobian[1][1],
            )?)?,
        ],
        [
            zero.subtract(linear_combination(
                inverse[1][0],
                jacobian[0][0],
                inverse[1][1],
                jacobian[1][0],
            )?)?,
            one.subtract(linear_combination(
                inverse[1][0],
                jacobian[0][1],
                inverse[1][1],
                jacobian[1][1],
            )?)?,
        ],
    ];
    // The two endpoint differences have opposite signs. Their interval hull,
    // not their interval sum, is the centered parameter box.
    let delta = [
        Interval::hull([
            Interval::exact(first.start)?.subtract(Interval::exact(first_mid)?)?,
            Interval::exact(first.end)?.subtract(Interval::exact(first_mid)?)?,
        ])?,
        Interval::hull([
            Interval::exact(second.start)?.subtract(Interval::exact(second_mid)?)?,
            Interval::exact(second.end)?.subtract(Interval::exact(second_mid)?)?,
        ])?,
    ];
    let image = [
        corrected[0]
            .add(matrix[0][0].multiply(delta[0])?)?
            .add(matrix[0][1].multiply(delta[1])?)?,
        corrected[1]
            .add(matrix[1][0].multiply(delta[0])?)?
            .add(matrix[1][1].multiply(delta[1])?)?,
    ];
    if !(image[0].lower() > first.start
        && image[0].upper() < first.end
        && image[1].lower() > second.start
        && image[1].upper() < second.end)
    {
        return Ok(None);
    }

    let determinant_interval = jacobian[0][0]
        .multiply(jacobian[1][1])?
        .subtract(jacobian[0][1].multiply(jacobian[1][0])?)?;
    let determinant_lower = determinant_interval.absolute_lower_bound();
    if determinant_lower == 0.0 {
        return Ok(None);
    }
    let first_parameter = image[0].lower() * 0.5 + image[0].upper() * 0.5;
    let second_parameter = image[1].lower() * 0.5 + image[1].upper() * 0.5;
    let first_value = bspline_jet2(first_curve, first_parameter)?.point;
    let second_value = bspline_jet2(second_curve, second_parameter)?.point;
    let residual = distance_between_point_intervals_upper(
        point_intervals(first_value)?,
        point_intervals(second_value)?,
        2,
    )?;
    Ok(Some(TransverseCurveIntersection2 {
        first_parameter: ParameterInterval {
            start: image[0].lower(),
            end: image[0].upper(),
        },
        second_parameter: ParameterInterval {
            start: image[1].lower(),
            end: image[1].upper(),
        },
        point: Point2::new(
            first_value.x * 0.5 + second_value.x * 0.5,
            first_value.y * 0.5 + second_value.y * 0.5,
        ),
        residual_upper_bound: residual,
        jacobian_determinant_lower_bound: determinant_lower,
    }))
}

fn point_intervals(point: Point2) -> GeomResult<[Interval; 3]> {
    Ok([
        Interval::exact(point.x)?,
        Interval::exact(point.y)?,
        Interval::exact(0.0)?,
    ])
}

fn linear_combination(
    left_scalar: Scalar,
    left: Interval,
    right_scalar: Scalar,
    right: Interval,
) -> GeomResult<Interval> {
    Interval::exact(left_scalar)?
        .multiply(left)?
        .add(Interval::exact(right_scalar)?.multiply(right)?)
}

fn classify_lines(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    visited_nodes: u32,
    boxes: Vec<CurvePairParameterBox>,
) -> GeomResult<CertifiedCurveIntersection2> {
    let [a, b] = [first.control_points[0], first.control_points[1]];
    let [c, d] = [second.control_points[0], second.control_points[1]];
    let signs = [sign(a, b, c), sign(a, b, d), sign(c, d, a), sign(c, d, b)];
    let first_collinear = signs[0] == Sign::Zero && signs[1] == Sign::Zero;
    if first_collinear {
        let classification = if boxes_overlap(a, b, c, d) {
            CurveIntersectionDegeneracy::Overlap
        } else {
            return Ok(CertifiedCurveIntersection2::Complete {
                intersections: Vec::new(),
                visited_nodes,
            });
        };
        return Ok(CertifiedCurveIntersection2::Degenerate {
            classification,
            candidate_boxes: boxes,
            visited_nodes,
        });
    }
    let intersects = !same_strict_sign(signs[0], signs[1])
        && !same_strict_sign(signs[2], signs[3])
        && boxes_overlap(a, b, c, d);
    if !intersects {
        return Ok(CertifiedCurveIntersection2::Complete {
            intersections: Vec::new(),
            visited_nodes,
        });
    }

    let r = b - a;
    let s = d - c;
    let determinant = r.x * s.y - r.y * s.x;
    let offset = c - a;
    let t = ((offset.x * s.y - offset.y * s.x) / determinant).clamp(0.0, 1.0);
    let u = ((offset.x * r.y - offset.y * r.x) / determinant).clamp(0.0, 1.0);
    let first_parameter = native_parameter(first, t);
    let second_parameter = native_parameter(second, u);
    let first_point = bspline_jet2(first, first_parameter)?.point;
    let second_point = bspline_jet2(second, second_parameter)?.point;
    let residual = distance_between_point_intervals_upper(
        point_intervals(first_point)?,
        point_intervals(second_point)?,
        2,
    )?;
    let point = Point2::new(
        first_point.x * 0.5 + second_point.x * 0.5,
        first_point.y * 0.5 + second_point.y * 0.5,
    );

    Ok(CertifiedCurveIntersection2::Complete {
        intersections: vec![TransverseCurveIntersection2 {
            first_parameter: domain(first),
            second_parameter: domain(second),
            point,
            residual_upper_bound: residual,
            jacobian_determinant_lower_bound: determinant_lower(a, b, c, d)?,
        }],
        visited_nodes,
    })
}

fn determinant_lower(a: Point2, b: Point2, c: Point2, d: Point2) -> GeomResult<Scalar> {
    let rx = Interval::exact(b.x)?.subtract(Interval::exact(a.x)?)?;
    let ry = Interval::exact(b.y)?.subtract(Interval::exact(a.y)?)?;
    let sx = Interval::exact(d.x)?.subtract(Interval::exact(c.x)?)?;
    let sy = Interval::exact(d.y)?.subtract(Interval::exact(c.y)?)?;
    Ok(rx
        .multiply(sy)?
        .subtract(ry.multiply(sx)?)?
        .absolute_lower_bound())
}

fn structural_start_tangency(line: &BSplineCurve2, quadratic: &BSplineCurve2) -> bool {
    if !linear_polynomial(line)
        || quadratic.degree != 2
        || quadratic.weights.is_some()
        || quadratic.knots.len() != 2
        || quadratic.control_points.len() != 3
    {
        return false;
    }
    let [a, b] = [line.control_points[0], line.control_points[1]];
    let q = &quadratic.control_points;
    a == q[0] && q[1] != a && sign(a, b, q[1]) == Sign::Zero && sign(a, b, q[2]) != Sign::Zero
}

fn linear_polynomial(curve: &BSplineCurve2) -> bool {
    curve.degree == 1
        && curve.weights.is_none()
        && curve.knots.len() == 2
        && curve.control_points.len() == 2
}

fn sign(a: Point2, b: Point2, c: Point2) -> Sign {
    orient2d(a, b, c).sign().expect("orient2d always escalates")
}

fn same_strict_sign(a: Sign, b: Sign) -> bool {
    a != Sign::Zero && a == b
}

fn boxes_overlap(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let overlap = |a0: Scalar, a1: Scalar, b0: Scalar, b1: Scalar| {
        a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1)
    };
    overlap(a.x, b.x, c.x, d.x) && overlap(a.y, b.y, c.y, d.y)
}

fn native_parameter(curve: &BSplineCurve2, unit: Scalar) -> Scalar {
    curve.knots[0] * (1.0 - unit) + curve.knots[curve.knots.len() - 1] * unit
}

fn domain(curve: &BSplineCurve2) -> ParameterInterval {
    ParameterInterval {
        start: curve.knots[0],
        end: curve.knots[curve.knots.len() - 1],
    }
}

fn product_boxes(
    first: &[crate::certified_bezier::Cell],
    second: &[crate::certified_bezier::Cell],
) -> Vec<CurvePairParameterBox> {
    first
        .iter()
        .flat_map(|a| {
            second.iter().map(move |b| CurvePairParameterBox {
                first: ParameterInterval {
                    start: a.start,
                    end: a.end,
                },
                second: ParameterInterval {
                    start: b.start,
                    end: b.end,
                },
            })
        })
        .collect()
}
