//! Certified planar curve/curve relationship classification.

use crate::certified_bezier::{distance_between_point_intervals_upper, Interval};
use crate::certified_projection::{CurvePairParameterBox, ParameterInterval};
use crate::certified_refinement::{piecewise_bezier_cells, RefinementBudget};
use axiolid_contracts::{GeomError, GeomResult, Sign};
use axiolid_core::{Point2, Scalar};
use axiolid_curve::BSplineCurve2;
use axiolid_evaluate::curve::bspline_jet2;
use axiolid_predicates::orient2d;

const MAX_CERTIFIED_CURVE_INTERSECTION_NODES: u32 = 100_000;
const MAX_CERTIFIED_CURVE_INTERSECTION_DEPTH: u16 = 64;

/// Accuracy and work policy for certified planar root isolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedCurveIntersectionOptions {
    parameter_tolerance: Scalar,
    max_nodes: u32,
    max_depth: u16,
}

impl CertifiedCurveIntersectionOptions {
    /// Construct a finite, non-vacuous root-isolation policy.
    ///
    /// `max_nodes` must be at most 100,000 and `max_depth` at most 64 so caller
    /// policy cannot authorize process-sized allocations.
    pub fn new(parameter_tolerance: Scalar, max_nodes: u32, max_depth: u16) -> GeomResult<Self> {
        if !parameter_tolerance.is_finite()
            || parameter_tolerance <= 0.0
            || max_nodes == 0
            || max_nodes > MAX_CERTIFIED_CURVE_INTERSECTION_NODES
            || max_depth == 0
            || max_depth > MAX_CERTIFIED_CURVE_INTERSECTION_DEPTH
        {
            return Err(GeomError::InvalidInput(
                "curve-intersection tolerance must be finite and positive; max_nodes must be in 1..=100000 and max_depth in 1..=64".to_owned(),
            ));
        }
        Ok(Self {
            parameter_tolerance,
            max_nodes,
            max_depth,
        })
    }

    /// Required maximum native-parameter width of each certified root interval.
    pub const fn parameter_tolerance(self) -> Scalar {
        self.parameter_tolerance
    }

    /// Maximum refinement work and generated product cells.
    pub const fn max_nodes(self) -> u32 {
        self.max_nodes
    }

    /// Maximum subdivision or Krawczyk-contraction depth per product cell.
    pub const fn max_depth(self) -> u16 {
        self.max_depth
    }
}

/// One isolated transverse planar intersection.
#[non_exhaustive]
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

/// One classified non-transverse contact, owned by exactly one parameter box.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassifiedCurveContact2 {
    /// Product-domain box that owns this contact.
    pub parameters: CurvePairParameterBox,
    /// Proven structural class, or `Unresolved` when no proof completed.
    pub classification: CurveIntersectionDegeneracy,
}

/// Singular or not-yet-isolated curve relationship.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveIntersectionDegeneracy {
    /// A structurally proven contact involving a zero-length curve.
    PointContact,
    /// A structurally proven endpoint tangency.
    Tangency,
    /// A structurally proven positive-dimensional overlap.
    Overlap,
    /// Candidate boxes remain but no supported proof classified them.
    Unresolved,
    /// A transverse contact proven to sit on a boundary shared by adjacent cells.
    ///
    /// Strict-interior Krawczyk isolation cannot prove a root that lies exactly
    /// on a cell edge, so a crossing at an interior knot would otherwise be
    /// reported as an unresolved fragment per incident cell. This class states
    /// the positive fact instead: the residual is not excluded and the tangents
    /// are certainly NOT parallel, so a transverse crossing exists in the fused
    /// boundary box and is owned by it exactly once.
    BoundaryCrossing,
}

/// Planar curve/curve classification under the implemented proof paths.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedCurveIntersection2 {
    /// Every product-domain cell was excluded or represented by one transverse root.
    /// Each returned parameter interval is no wider than the requested tolerance.
    Complete {
        /// Isolated transverse roots. Empty means certified disjointness.
        intersections: Vec<TransverseCurveIntersection2>,
        /// Generated root cells inspected by the classifier.
        visited_nodes: u32,
    },
    /// A singular or currently unsupported candidate was not called transverse.
    Degenerate {
        /// Strongest class proven across `contacts`.
        ///
        /// Precedence is `Overlap` > `Tangency` > `PointContact` > `Unresolved`:
        /// a caller that only wants one verdict gets the most structural one,
        /// while `contacts` retains which box proved what.
        classification: CurveIntersectionDegeneracy,
        /// Per-box classification. Boxes are deduplicated and boundary-owned,
        /// so a root on a shared cell endpoint appears exactly once.
        contacts: Vec<ClassifiedCurveContact2>,
        /// Generated root cells inspected by the classifier.
        visited_nodes: u32,
    },
}

/// Classify intersections of two clamped planar B-spline curves.
///
/// The complete proof path covers exact-sign single-span polynomial lines,
/// zero-length line `PointContact`s, and strict-interior Krawczyk isolation of
/// transverse polynomial or positive-weight rational Bézier roots. Unsupported
/// singular, tangential, coincident, seam, or proof-insufficient candidates return
/// [`CurveIntersectionDegeneracy::Unresolved`] rather than an uncertified root.
pub fn intersect_curve2_certified(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    options: CertifiedCurveIntersectionOptions,
) -> GeomResult<CertifiedCurveIntersection2> {
    let mut budget =
        RefinementBudget::new(options.max_nodes(), "certified curve intersection budget");
    let first_cells = piecewise_bezier_cells(first, |p| [p.x, p.y, 0.0], &mut budget)?;
    let second_cells = piecewise_bezier_cells(second, |p| [p.x, p.y, 0.0], &mut budget)?;
    let count = first_cells.len().checked_mul(second_cells.len());
    let visited_nodes = count
        .and_then(|value| u32::try_from(value).ok())
        .filter(|&value| value <= options.max_nodes())
        .ok_or(axiolid_contracts::GeomError::BudgetExceeded {
            resource: "certified curve intersection budget",
        })?;
    if first == second {
        let mut contacts = Vec::new();
        contacts
            .try_reserve_exact(first_cells.len())
            .map_err(|_| GeomError::BudgetExceeded {
                resource: "certified curve intersection result allocation",
            })?;
        let classification = if has_control_point_extent(first) {
            CurveIntersectionDegeneracy::Overlap
        } else {
            CurveIntersectionDegeneracy::PointContact
        };
        for (first_cell, second_cell) in first_cells.iter().zip(&second_cells) {
            push_contact(
                &mut contacts,
                pair_box(first_cell, second_cell),
                classification,
            )?;
        }
        return Ok(degenerate(contacts, visited_nodes));
    }
    if structural_start_tangency(first, second) || structural_start_tangency(second, first) {
        let mut contacts = Vec::new();
        push_contact(
            &mut contacts,
            pair_box(&first_cells[0], &second_cells[0]),
            CurveIntersectionDegeneracy::Tangency,
        )?;
        return Ok(degenerate(contacts, visited_nodes));
    }
    if linear_polynomial(first) && linear_polynomial(second) {
        return classify_lines(
            first,
            second,
            visited_nodes,
            vec![pair_box(&first_cells[0], &second_cells[0])],
            options,
        );
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

#[derive(Debug, Clone, Copy)]
struct PendingCurvePair {
    first_index: usize,
    second_index: usize,
    parameters: CurvePairParameterBox,
    depth: u16,
}

fn classify_nonlinear(
    first: &BSplineCurve2,
    second: &BSplineCurve2,
    first_cells: Vec<crate::certified_bezier::Cell>,
    second_cells: Vec<crate::certified_bezier::Cell>,
    mut visited_nodes: u32,
    options: CertifiedCurveIntersectionOptions,
) -> GeomResult<CertifiedCurveIntersection2> {
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(usize::from(options.max_depth()) + 1)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "certified curve intersection pending allocation",
        })?;
    let mut intersections = Vec::new();
    let mut contacts = Vec::new();

    for (first_index, first_base) in first_cells.iter().enumerate() {
        for (second_index, second_base) in second_cells.iter().enumerate() {
            pending.push(PendingCurvePair {
                first_index,
                second_index,
                parameters: pair_box(first_base, second_base),
                depth: 0,
            });
            while let Some(current) = pending.pop() {
                let first_cell = first_cells[current.first_index]
                    .restrict(current.parameters.first.start, current.parameters.first.end)?;
                let second_cell = second_cells[current.second_index].restrict(
                    current.parameters.second.start,
                    current.parameters.second.end,
                )?;
                let depth = current.depth;
                if residual_excludes_zero(&first_cell, &second_cell)? {
                    continue;
                }
                if let Some(root) = krawczyk_root(first, second, &first_cell, &second_cell)? {
                    if root.first_parameter.end - root.first_parameter.start
                        <= options.parameter_tolerance()
                        && root.second_parameter.end - root.second_parameter.start
                            <= options.parameter_tolerance()
                    {
                        push_root(&mut intersections, root)?;
                        continue;
                    }
                    if depth >= options.max_depth() {
                        push_contact(
                            &mut contacts,
                            CurvePairParameterBox {
                                first: root.first_parameter,
                                second: root.second_parameter,
                            },
                            CurveIntersectionDegeneracy::Unresolved,
                        )?;
                        continue;
                    }
                    visited_nodes = visited_nodes
                        .checked_add(1)
                        .filter(|&count| count <= options.max_nodes())
                        .ok_or(GeomError::BudgetExceeded {
                            resource: "certified curve intersection budget",
                        })?;
                    pending.push(PendingCurvePair {
                        first_index: current.first_index,
                        second_index: current.second_index,
                        parameters: CurvePairParameterBox {
                            first: contract_interval(
                                &first_cell,
                                root.first_parameter,
                                options.parameter_tolerance(),
                            ),
                            second: contract_interval(
                                &second_cell,
                                root.second_parameter,
                                options.parameter_tolerance(),
                            ),
                        },
                        depth: depth.checked_add(1).ok_or_else(|| {
                            GeomError::Degenerate("curve-intersection depth overflow".to_owned())
                        })?,
                    });
                    continue;
                }
                if depth >= options.max_depth() {
                    push_contact(
                        &mut contacts,
                        pair_box(&first_cell, &second_cell),
                        classify_exhausted_box(&first_cell, &second_cell, first_base, second_base)?,
                    )?;
                    continue;
                }
                visited_nodes = visited_nodes
                    .checked_add(2)
                    .filter(|&count| count <= options.max_nodes())
                    .ok_or(axiolid_contracts::GeomError::BudgetExceeded {
                        resource: "certified curve intersection budget",
                    })?;
                if first_cell.end - first_cell.start >= second_cell.end - second_cell.start {
                    let (left, right) = split_parameter_interval(current.parameters.first)?;
                    pending.push(PendingCurvePair {
                        parameters: CurvePairParameterBox {
                            first: left,
                            ..current.parameters
                        },
                        depth: depth + 1,
                        ..current
                    });
                    pending.push(PendingCurvePair {
                        parameters: CurvePairParameterBox {
                            first: right,
                            ..current.parameters
                        },
                        depth: depth + 1,
                        ..current
                    });
                } else {
                    let (left, right) = split_parameter_interval(current.parameters.second)?;
                    pending.push(PendingCurvePair {
                        parameters: CurvePairParameterBox {
                            second: left,
                            ..current.parameters
                        },
                        depth: depth + 1,
                        ..current
                    });
                    pending.push(PendingCurvePair {
                        parameters: CurvePairParameterBox {
                            second: right,
                            ..current.parameters
                        },
                        depth: depth + 1,
                        ..current
                    });
                }
            }
        }
    }

    if contacts.is_empty() {
        Ok(CertifiedCurveIntersection2::Complete {
            intersections,
            visited_nodes,
        })
    } else {
        Ok(degenerate(contacts, visited_nodes))
    }
}

/// Rank a class so a single summary verdict can be derived from many contacts.
///
/// A positive-dimensional overlap subsumes a tangency, which subsumes an
/// isolated point contact; `Unresolved` is weakest because it asserts nothing.
fn classification_rank(classification: CurveIntersectionDegeneracy) -> u8 {
    match classification {
        CurveIntersectionDegeneracy::Overlap => 4,
        CurveIntersectionDegeneracy::Tangency => 3,
        CurveIntersectionDegeneracy::BoundaryCrossing => 2,
        CurveIntersectionDegeneracy::PointContact => 1,
        CurveIntersectionDegeneracy::Unresolved => 0,
    }
}

/// Two parameter boxes describe the same contact.
///
/// Adjacent Bézier cells share an endpoint, so the same root is reachable from
/// both sides of a knot. Ownership is resolved by comparing the closed boxes
/// exactly: subdivision copies endpoints bit-for-bit rather than recomputing
/// them, so equal boxes really are the same contact and not two near-misses.
fn same_contact(left: &CurvePairParameterBox, right: &CurvePairParameterBox) -> bool {
    left.first.start == right.first.start
        && left.first.end == right.first.end
        && left.second.start == right.second.start
        && left.second.end == right.second.end
}

fn push_contact(
    contacts: &mut Vec<ClassifiedCurveContact2>,
    parameters: CurvePairParameterBox,
    classification: CurveIntersectionDegeneracy,
) -> GeomResult<()> {
    if let Some(existing) = contacts
        .iter_mut()
        .find(|contact| same_contact(&contact.parameters, &parameters))
    {
        // Keep the strongest proof for a box we have already claimed instead of
        // reporting the same contact twice under two different classes.
        if classification_rank(classification) > classification_rank(existing.classification) {
            existing.classification = classification;
        }
        return Ok(());
    }
    push_result(
        contacts,
        ClassifiedCurveContact2 {
            parameters,
            classification,
        },
    )
}

/// Collapse contacts that meet across a shared cell boundary.
///
/// Krawczyk proves roots in the STRICT interior of a box. A root lying exactly
/// on a boundary shared by adjacent product cells is therefore never proved
/// transverse: it fragments into up to four touching unresolved boxes, one per
/// incident cell. Reporting four contacts for one geometric root is exactly
/// the double-counting #5 forbids, so touching boxes of equal class are fused
/// into their hull and the fused box owns the contact.
fn fuse_touching(mut contacts: Vec<ClassifiedCurveContact2>) -> Vec<ClassifiedCurveContact2> {
    let mut fused: Vec<ClassifiedCurveContact2> = Vec::new();
    for contact in contacts.drain(..) {
        if let Some(existing) = fused.iter_mut().find(|owner| {
            // Overlap spans stay separate.
            owner.classification == contact.classification
                && owner.classification != CurveIntersectionDegeneracy::Overlap
                && intervals_touch(owner.parameters.first, contact.parameters.first)
                && intervals_touch(owner.parameters.second, contact.parameters.second)
        }) {
            existing.parameters = hull_box(existing.parameters, contact.parameters);
            continue;
        }
        fused.push(contact);
    }
    fused
}

fn hull_box(left: CurvePairParameterBox, right: CurvePairParameterBox) -> CurvePairParameterBox {
    CurvePairParameterBox {
        first: hull_interval(left.first, right.first),
        second: hull_interval(left.second, right.second),
    }
}

fn hull_interval(left: ParameterInterval, right: ParameterInterval) -> ParameterInterval {
    ParameterInterval {
        start: left.start.min(right.start),
        end: left.end.max(right.end),
    }
}

fn degenerate(
    contacts: Vec<ClassifiedCurveContact2>,
    visited_nodes: u32,
) -> CertifiedCurveIntersection2 {
    let contacts = fuse_touching(contacts);
    let classification = contacts
        .iter()
        .map(|contact| contact.classification)
        .max_by_key(|&class| classification_rank(class))
        .unwrap_or(CurveIntersectionDegeneracy::Unresolved);
    CertifiedCurveIntersection2::Degenerate {
        classification,
        contacts,
        visited_nodes,
    }
}

/// Record a transverse root, resolving boundary ownership.
///
/// Two adjacent product cells share a closed edge, so a root exactly on that
/// edge is isolated twice — once per cell. Both proofs are real; the root is
/// not. The first owner wins and the duplicate is dropped, so a shared-endpoint
/// crossing is reported exactly once (#5).
fn push_root(
    intersections: &mut Vec<TransverseCurveIntersection2>,
    root: TransverseCurveIntersection2,
) -> GeomResult<()> {
    if intersections
        .iter()
        .any(|existing| roots_overlap(existing, &root))
    {
        return Ok(());
    }
    push_result(intersections, root)
}

/// Two isolated roots enclose the same point.
///
/// Certified intervals are closed, so touching intervals (`a.end == b.start`)
/// are the shared-endpoint case that ownership must collapse. Disjoint
/// intervals are genuinely different roots and are both kept.
fn roots_overlap(
    left: &TransverseCurveIntersection2,
    right: &TransverseCurveIntersection2,
) -> bool {
    intervals_touch(left.first_parameter, right.first_parameter)
        && intervals_touch(left.second_parameter, right.second_parameter)
}

fn intervals_touch(left: ParameterInterval, right: ParameterInterval) -> bool {
    left.start <= right.end && right.start <= left.end
}

/// Classify a box that survived subdivision to the depth limit.
///
/// Reaching the limit means Krawczyk never proved a transverse root. For a
/// genuine transverse crossing that should not happen: transversality makes the
/// Jacobian invertible and the operator contractive. The usual cause is
/// TANGENCY — the tangent directions align, the determinant approaches zero,
/// and no contraction exists.
///
/// This is a positive proof, not an inference from failure. Over the WHOLE
/// box, conservative derivative interval hulls must bound the cross product
/// `d1 x d2` to an interval containing zero while the residual is not excluded.
/// Evaluating parallelism at the midpoints alone would be unsound: two float
/// midpoints of a transverse pair can be accidentally parallel. If the cross
/// product is certainly nonzero, or either derivative hull contains the zero
/// vector (a cusp or stationary parameterisation), the box stays `Unresolved`
/// rather than carrying a class the code cannot prove.
fn classify_exhausted_box(
    first: &crate::certified_bezier::Cell,
    second: &crate::certified_bezier::Cell,
    first_base: &crate::certified_bezier::Cell,
    second_base: &crate::certified_bezier::Cell,
) -> GeomResult<CurveIntersectionDegeneracy> {
    let first_derivative = first.derivative_intervals()?;
    let second_derivative = second.derivative_intervals()?;

    // A hull containing the zero vector cannot certify a direction at all.
    if derivative_hull_contains_zero(first_derivative)
        || derivative_hull_contains_zero(second_derivative)
    {
        return Ok(CurveIntersectionDegeneracy::Unresolved);
    }

    // 2D cross product over interval hulls: d1.x*d2.y - d1.y*d2.x.
    let cross = first_derivative[0]
        .multiply(second_derivative[1])?
        .subtract(first_derivative[1].multiply(second_derivative[0])?)?;
    if cross.contains_zero() {
        return Ok(CurveIntersectionDegeneracy::Tangency);
    }
    // The residual is not excluded and the tangents are certainly not parallel,
    // so a transverse crossing exists here. Strict-interior isolation simply
    // cannot name it, because it sits on the shared cell boundary.
    // `BoundaryCrossing` may only be claimed when the box actually reaches a
    // shared cell endpoint. Budget exhaustion in a cell interior proves
    // nothing and stays `Unresolved`.
    let on_boundary =
        touches_base_endpoint(first, first_base) || touches_base_endpoint(second, second_base);
    if on_boundary {
        return Ok(CurveIntersectionDegeneracy::BoundaryCrossing);
    }
    Ok(CurveIntersectionDegeneracy::Unresolved)
}

/// The refined box reaches an endpoint of its originating Bezier cell.
///
/// Subdivision copies endpoints bit-for-bit, so an exact comparison identifies
/// the shared-boundary case without a tolerance.
fn touches_base_endpoint(
    refined: &crate::certified_bezier::Cell,
    base: &crate::certified_bezier::Cell,
) -> bool {
    refined.start == base.start || refined.end == base.end
}

fn derivative_hull_contains_zero(derivative: [Interval; 3]) -> bool {
    derivative[0].contains_zero() && derivative[1].contains_zero()
}

fn push_result<T>(target: &mut Vec<T>, value: T) -> GeomResult<()> {
    target
        .try_reserve(1)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "certified curve intersection result allocation",
        })?;
    target.push(value);
    Ok(())
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

fn stable_start(cell: Scalar, root: Scalar, desired: Scalar) -> Scalar {
    if desired > cell {
        desired.min(root)
    } else {
        root
    }
}

fn stable_end(cell: Scalar, root: Scalar, desired: Scalar) -> Scalar {
    if desired < cell {
        desired.max(root)
    } else {
        root
    }
}

fn stable_contraction(
    cell: &crate::certified_bezier::Cell,
    root: ParameterInterval,
    tolerance: Scalar,
) -> ParameterInterval {
    let center = midpoint(root);
    let half = tolerance * 0.5;
    ParameterInterval {
        start: stable_start(cell.start, root.start, center - half),
        end: stable_end(cell.end, root.end, center + half),
    }
}

fn contract_interval(
    cell: &crate::certified_bezier::Cell,
    interval: ParameterInterval,
    tolerance: Scalar,
) -> ParameterInterval {
    if cell.end - cell.start <= tolerance {
        ParameterInterval {
            start: cell.start,
            end: cell.end,
        }
    } else {
        stable_contraction(cell, interval, tolerance)
    }
}

fn split_parameter_interval(
    interval: ParameterInterval,
) -> GeomResult<(ParameterInterval, ParameterInterval)> {
    let split = midpoint(interval);
    if split <= interval.start || split >= interval.end {
        return Err(GeomError::Degenerate(
            "certified curve intersection parameter split did not advance".to_owned(),
        ));
    }
    Ok((
        ParameterInterval {
            start: interval.start,
            end: split,
        },
        ParameterInterval {
            start: split,
            end: interval.end,
        },
    ))
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
    options: CertifiedCurveIntersectionOptions,
) -> GeomResult<CertifiedCurveIntersection2> {
    let [a, b] = [first.control_points[0], first.control_points[1]];
    let [c, d] = [second.control_points[0], second.control_points[1]];
    if a == b || c == d {
        let intersects = match (a == b, c == d) {
            (true, true) => a == c,
            (true, false) => point_on_segment(a, c, d),
            (false, true) => point_on_segment(c, a, b),
            (false, false) => unreachable!("a degenerate segment was already established"),
        };
        return Ok(if intersects {
            let mut contacts = Vec::new();
            for owned in boxes {
                push_contact(
                    &mut contacts,
                    owned,
                    CurveIntersectionDegeneracy::PointContact,
                )?;
            }
            degenerate(contacts, visited_nodes)
        } else {
            CertifiedCurveIntersection2::Complete {
                intersections: Vec::new(),
                visited_nodes,
            }
        });
    }
    let signs = [sign(a, b, c), sign(a, b, d), sign(c, d, a), sign(c, d, b)];
    let first_collinear = signs[0] == Sign::Zero && signs[1] == Sign::Zero;
    if first_collinear {
        let classification = if boxes_overlap_positive(a, b, c, d) {
            CurveIntersectionDegeneracy::Overlap
        } else if boxes_overlap(a, b, c, d) {
            CurveIntersectionDegeneracy::Tangency
        } else {
            return Ok(CertifiedCurveIntersection2::Complete {
                intersections: Vec::new(),
                visited_nodes,
            });
        };
        return Ok(degenerate(
            {
                let mut contacts = Vec::new();
                for owned in boxes {
                    push_contact(&mut contacts, owned, classification)?;
                }
                contacts
            },
            visited_nodes,
        ));
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

    let rx = Interval::exact(b.x)?.subtract(Interval::exact(a.x)?)?;
    let ry = Interval::exact(b.y)?.subtract(Interval::exact(a.y)?)?;
    let sx = Interval::exact(d.x)?.subtract(Interval::exact(c.x)?)?;
    let sy = Interval::exact(d.y)?.subtract(Interval::exact(c.y)?)?;
    let determinant = rx.multiply(sy)?.subtract(ry.multiply(sx)?)?;
    let ox = Interval::exact(c.x)?.subtract(Interval::exact(a.x)?)?;
    let oy = Interval::exact(c.y)?.subtract(Interval::exact(a.y)?)?;
    let t = ox
        .multiply(sy)?
        .subtract(oy.multiply(sx)?)?
        .divide_nonzero(determinant)?;
    let u = ox
        .multiply(ry)?
        .subtract(oy.multiply(rx)?)?
        .divide_nonzero(determinant)?;
    let first_interval = native_parameter_interval(first, t)?;
    let second_interval = native_parameter_interval(second, u)?;
    if !intervals_resolved(
        first_interval,
        second_interval,
        options.parameter_tolerance(),
    ) {
        return Ok(unresolved_intervals(
            first_interval,
            second_interval,
            visited_nodes,
        ));
    }
    let first_parameter = midpoint(first_interval);
    let second_parameter = midpoint(second_interval);
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
            first_parameter: first_interval,
            second_parameter: second_interval,
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

fn has_control_point_extent(curve: &BSplineCurve2) -> bool {
    curve
        .control_points
        .first()
        .is_some_and(|first| curve.control_points.iter().any(|point| point != first))
}

fn point_on_segment(point: Point2, start: Point2, end: Point2) -> bool {
    sign(start, end, point) == Sign::Zero && boxes_overlap(point, point, start, end)
}

fn boxes_overlap(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let overlap = |a0: Scalar, a1: Scalar, b0: Scalar, b1: Scalar| {
        a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1)
    };
    overlap(a.x, b.x, c.x, d.x) && overlap(a.y, b.y, c.y, d.y)
}

fn boxes_overlap_positive(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let width = |a0: Scalar, a1: Scalar, b0: Scalar, b1: Scalar| {
        a0.max(a1).min(b0.max(b1)) - a0.min(a1).max(b0.min(b1))
    };
    boxes_overlap(a, b, c, d)
        && (width(a.x, b.x, c.x, d.x) > 0.0 || width(a.y, b.y, c.y, d.y) > 0.0)
}

fn midpoint(interval: ParameterInterval) -> Scalar {
    interval.start * 0.5 + interval.end * 0.5
}

fn intervals_resolved(
    first: ParameterInterval,
    second: ParameterInterval,
    tolerance: Scalar,
) -> bool {
    first.end - first.start <= tolerance && second.end - second.start <= tolerance
}

fn unresolved_box(box_: CurvePairParameterBox, visited_nodes: u32) -> CertifiedCurveIntersection2 {
    CertifiedCurveIntersection2::Degenerate {
        classification: CurveIntersectionDegeneracy::Unresolved,
        contacts: vec![ClassifiedCurveContact2 {
            parameters: box_,
            classification: CurveIntersectionDegeneracy::Unresolved,
        }],
        visited_nodes,
    }
}

fn unresolved_intervals(
    first: ParameterInterval,
    second: ParameterInterval,
    visited: u32,
) -> CertifiedCurveIntersection2 {
    unresolved_box(CurvePairParameterBox { first, second }, visited)
}

fn checked_parameter_interval(start: Scalar, end: Scalar) -> GeomResult<ParameterInterval> {
    (start <= end)
        .then_some(ParameterInterval { start, end })
        .ok_or_else(|| {
            GeomError::Degenerate("certified line parameter interval is empty".to_owned())
        })
}

fn native_interval(bounds: ParameterInterval, value: Interval) -> GeomResult<ParameterInterval> {
    checked_parameter_interval(
        value.lower().max(bounds.start),
        value.upper().min(bounds.end),
    )
}

fn native_parameter_interval_inner(
    bounds: ParameterInterval,
    unit: Interval,
) -> GeomResult<ParameterInterval> {
    let span = Interval::exact(bounds.end)?.subtract(Interval::exact(bounds.start)?)?;
    native_interval(
        bounds,
        Interval::exact(bounds.start)?.add(unit.multiply(span)?)?,
    )
}

fn native_parameter_interval(
    curve: &BSplineCurve2,
    unit: Interval,
) -> GeomResult<ParameterInterval> {
    native_parameter_interval_inner(domain(curve), unit)
}

fn domain(curve: &BSplineCurve2) -> ParameterInterval {
    ParameterInterval {
        start: curve.knots[0],
        end: curve.knots[curve.knots.len() - 1],
    }
}

#[cfg(test)]
mod pending_storage_tests {
    use super::{CurvePairParameterBox, PendingCurvePair};

    #[test]
    fn pending_curve_pairs_store_only_indices_parameters_and_depth() {
        let raw = 2 * size_of::<usize>() + size_of::<CurvePairParameterBox>() + size_of::<u16>();
        let alignment = align_of::<PendingCurvePair>();
        let expected = raw.next_multiple_of(alignment);
        assert_eq!(size_of::<PendingCurvePair>(), expected);
    }
}
