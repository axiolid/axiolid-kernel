//! Explicit cyclic B-spline surface semantics.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar};
use axiolid_reference::surface::{bspline_jet, SurfaceJet};
use axiolid_surface::BSplineSurface;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Axis {
    domain: (Scalar, Scalar),
    degree: usize,
    unique_count: usize,
    periodic: bool,
    seam_continuity_order: Option<u16>,
}

/// An owned tensor-product B-spline surface with validated cyclic axes.
///
/// A periodic axis uses the standard expanded periodic representation: the
/// final `degree` control rows or columns exactly repeat the first `degree`,
/// and the expanded knot vector repeats after one active period. Parameters on
/// that axis are evaluated modulo the half-open active domain. The wrapped
/// neutral [`BSplineSurface`] remains available for serialization and for
/// algorithms that understand the same explicit representation.
///
/// Construction is additive: the neutral evaluator continues to clamp native
/// parameters and does not infer cyclic behavior from closure metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicBSplineSurface {
    surface: BSplineSurface,
    u: Axis,
    v: Axis,
}

impl PeriodicBSplineSurface {
    /// Validate and own an explicitly periodic surface.
    ///
    /// At least one neutral closure flag must be set. Every declared periodic
    /// axis must satisfy exact control/weight aliasing and periodic knot
    /// extension invariants. Rational weights must be finite and positive.
    pub fn new(surface: BSplineSurface) -> GeomResult<Self> {
        if !surface.u_closed && !surface.v_closed {
            return Err(invalid(
                "PeriodicBSplineSurface requires at least one declared periodic axis",
            ));
        }
        validate_periodic_multiplicities(
            &surface.u_multiplicities,
            surface.u_degree,
            surface.u_closed,
            "U",
        )?;
        validate_periodic_multiplicities(
            &surface.v_multiplicities,
            surface.v_degree,
            surface.v_closed,
            "V",
        )?;
        let (u_count, v_count) = validate_control_net(&surface)?;
        let u_knots = expand_axis(
            &surface.u_knots,
            &surface.u_multiplicities,
            surface.u_degree,
            u_count,
            "periodic surface U axis",
        )?;
        let v_knots = expand_axis(
            &surface.v_knots,
            &surface.v_multiplicities,
            surface.v_degree,
            v_count,
            "periodic surface V axis",
        )?;
        let u = validate_axis(&u_knots, surface.u_degree, u_count, surface.u_closed, "U")?;
        let v = validate_axis(&v_knots, surface.v_degree, v_count, surface.v_closed, "V")?;
        validate_aliases(&surface, u, v)?;

        let u_mid = midpoint(u.domain)?;
        let v_mid = midpoint(v.domain)?;
        bspline_jet(&surface, u_mid, v_mid)?;
        Ok(Self { surface, u, v })
    }

    /// Borrow the canonical expanded neutral representation.
    pub const fn as_bspline_surface(&self) -> &BSplineSurface {
        &self.surface
    }

    /// Consume the capability type and return its canonical expanded value.
    pub fn into_bspline_surface(self) -> BSplineSurface {
        self.surface
    }

    /// Whether the U axis has explicit cyclic semantics.
    pub const fn u_is_periodic(&self) -> bool {
        self.u.periodic
    }

    /// Whether the V axis has explicit cyclic semantics.
    pub const fn v_is_periodic(&self) -> bool {
        self.v.periodic
    }

    /// Native active U domain; periodic evaluation treats it as half-open.
    pub const fn u_domain(&self) -> (Scalar, Scalar) {
        self.u.domain
    }

    /// Native active V domain; periodic evaluation treats it as half-open.
    pub const fn v_domain(&self) -> (Scalar, Scalar) {
        self.v.domain
    }

    /// Algebraic continuity order across the U seam, or `None` when U is open.
    #[must_use]
    pub const fn u_seam_continuity_order(&self) -> Option<u16> {
        self.u.seam_continuity_order
    }

    /// Algebraic continuity order across the V seam, or `None` when V is open.
    #[must_use]
    pub const fn v_seam_continuity_order(&self) -> Option<u16> {
        self.v.seam_continuity_order
    }

    /// Number of topologically unique U control rows.
    pub const fn unique_u_control_count(&self) -> usize {
        self.u.unique_count
    }

    /// Number of topologically unique V control columns.
    pub const fn unique_v_control_count(&self) -> usize {
        self.v.unique_count
    }

    /// Canonicalize finite parameters on periodic axes.
    ///
    /// Non-periodic parameters are left unchanged and retain the neutral
    /// evaluator's established clamping behavior. Periodic offsets whose
    /// binary64 spacing cannot resolve one complete period are refused rather
    /// than reduced modulo an under-resolved quotient coordinate.
    pub fn wrap_parameters(&self, u: Scalar, v: Scalar) -> GeomResult<(Scalar, Scalar)> {
        Ok((wrap_axis(u, self.u)?, wrap_axis(v, self.v)?))
    }

    /// Evaluate a point after periodic parameter canonicalization.
    pub fn point(&self, u: Scalar, v: Scalar) -> GeomResult<Point3> {
        Ok(self.jet(u, v)?.point)
    }

    /// Evaluate the full second-order jet after periodic canonicalization.
    pub fn jet(&self, u: Scalar, v: Scalar) -> GeomResult<SurfaceJet> {
        let (u, v) = self.wrap_parameters(u, v)?;
        bspline_jet(&self.surface, u, v)
    }

    /// Replace one topologically unique control point and all seam aliases.
    ///
    /// Indices address the unique cyclic net, never the duplicated expansion.
    /// Doubly periodic corner aliases are updated atomically with the primary
    /// control. Knot topology and rational weights are unchanged.
    pub fn set_control_point(&mut self, u: usize, v: usize, point: Point3) -> GeomResult<()> {
        if !point.is_finite() {
            return Err(invalid("periodic surface control point must be finite"));
        }
        let u_aliases = aliases(u, self.u, "U")?;
        let v_aliases = aliases(v, self.v, "V")?;
        for &row in &u_aliases {
            for &column in &v_aliases {
                self.surface.control_points[row][column] = point;
            }
        }
        Ok(())
    }

    /// Replace a control point using signed cyclic indices.
    ///
    /// Periodic-axis indices wrap modulo the topologically unique control count,
    /// so `-1` and `unique_count - 1` edit the same seam-adjacent control. Indices
    /// on non-periodic axes remain strict.
    pub fn set_control_point_wrapped(&mut self, u: i64, v: i64, point: Point3) -> GeomResult<()> {
        let u = wrapped_control_index(u, self.u, "U")?;
        let v = wrapped_control_index(v, self.v, "V")?;
        self.set_control_point(u, v, point)
    }

    /// Replace one topologically unique rational weight and all seam aliases.
    ///
    /// Polynomial surfaces reject this operation. The replacement must remain
    /// finite and strictly positive so rational certification stays well posed.
    pub fn set_weight(&mut self, u: usize, v: usize, weight: Scalar) -> GeomResult<()> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(invalid(
                "periodic surface weight must be finite and positive",
            ));
        }
        let u_aliases = aliases(u, self.u, "U")?;
        let v_aliases = aliases(v, self.v, "V")?;
        let weights = self
            .surface
            .weights
            .as_mut()
            .ok_or_else(|| invalid("polynomial periodic surface has no rational weights"))?;
        for &row in &u_aliases {
            for &column in &v_aliases {
                weights[row][column] = weight;
            }
        }
        Ok(())
    }

    /// Replace a rational weight using signed cyclic indices.
    pub fn set_weight_wrapped(&mut self, u: i64, v: i64, weight: Scalar) -> GeomResult<()> {
        let u = wrapped_control_index(u, self.u, "U")?;
        let v = wrapped_control_index(v, self.v, "V")?;
        self.set_weight(u, v, weight)
    }
}

fn wrapped_control_index(index: i64, axis: Axis, label: &str) -> GeomResult<usize> {
    if axis.periodic {
        let count = i64::try_from(axis.unique_count)
            .map_err(|_| invalid(&format!("{label} unique control count does not fit i64")))?;
        return usize::try_from(index.rem_euclid(count))
            .map_err(|_| invalid(&format!("{label} wrapped control index does not fit usize")));
    }
    let index = usize::try_from(index)
        .map_err(|_| invalid(&format!("{label} control index is negative")))?;
    if index >= axis.unique_count {
        return Err(invalid(&format!(
            "{label} control index {index} is outside 0..{}",
            axis.unique_count
        )));
    }
    Ok(index)
}

fn validate_periodic_multiplicities(
    multiplicities: &[u32],
    degree: u16,
    periodic: bool,
    label: &str,
) -> GeomResult<()> {
    if periodic
        && multiplicities
            .iter()
            .any(|&value| value == 0 || value > u32::from(degree))
    {
        return Err(invalid(&format!(
            "periodic surface {label} multiplicities must be in 1..={degree}"
        )));
    }
    Ok(())
}

fn validate_control_net(surface: &BSplineSurface) -> GeomResult<(usize, usize)> {
    let u_count = surface.control_points.len();
    let v_count = surface.control_points.first().map_or(0, Vec::len);
    if u_count == 0 || v_count == 0 {
        return Err(invalid("periodic surface control net must be nonempty"));
    }
    if surface
        .control_points
        .iter()
        .any(|row| row.len() != v_count)
    {
        return Err(invalid("periodic surface control net must be rectangular"));
    }
    if surface
        .control_points
        .iter()
        .flatten()
        .any(|point| !point.is_finite())
    {
        return Err(invalid("periodic surface control points must be finite"));
    }
    if let Some(weights) = &surface.weights {
        if weights.len() != u_count || weights.iter().any(|row| row.len() != v_count) {
            return Err(invalid(
                "periodic surface weight net must match the control net",
            ));
        }
        if weights
            .iter()
            .flatten()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
        {
            return Err(invalid(
                "periodic surface weights must be finite and positive",
            ));
        }
    }
    Ok((u_count, v_count))
}

fn expand_axis(
    knots: &[Scalar],
    multiplicities: &[u32],
    degree: u16,
    count: usize,
    label: &str,
) -> GeomResult<Vec<Scalar>> {
    if knots.len() != multiplicities.len() || knots.len() < 2 {
        return Err(invalid(&format!(
            "{label} compact knot data is inconsistent"
        )));
    }
    if knots.iter().any(|knot| !knot.is_finite()) || knots.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(invalid(&format!(
            "{label} knots must be finite and strictly increasing"
        )));
    }
    let degree = usize::from(degree);
    if degree == 0 || count <= degree {
        return Err(invalid(&format!("{label} degree/control count is invalid")));
    }
    let expected = count
        .checked_add(degree)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| invalid(&format!("{label} size overflows usize")))?;
    let maximum = degree
        .checked_add(1)
        .ok_or_else(|| invalid(&format!("{label} degree overflows usize")))?;
    let mut total = 0usize;
    for &multiplicity in multiplicities {
        let multiplicity = usize::try_from(multiplicity)
            .map_err(|_| invalid(&format!("{label} multiplicity does not fit usize")))?;
        if multiplicity == 0 || multiplicity > maximum {
            return Err(invalid(&format!(
                "{label} multiplicity is outside 1..={maximum}"
            )));
        }
        total = total
            .checked_add(multiplicity)
            .ok_or_else(|| invalid(&format!("{label} multiplicity sum overflows usize")))?;
        if total > expected {
            return Err(invalid(&format!("{label} has too many expanded knots")));
        }
    }
    if total != expected {
        return Err(invalid(&format!(
            "{label} has {total} expanded knots, expected {expected}"
        )));
    }
    let mut expanded = Vec::new();
    expanded
        .try_reserve_exact(expected)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "periodic surface knot expansion",
        })?;
    for (&knot, &multiplicity) in knots.iter().zip(multiplicities) {
        let multiplicity = usize::try_from(multiplicity)
            .map_err(|_| invalid(&format!("{label} multiplicity overflows usize")))?;
        expanded.extend(core::iter::repeat_n(knot, multiplicity));
    }
    Ok(expanded)
}

fn validate_axis(
    knots: &[Scalar],
    degree: u16,
    count: usize,
    periodic: bool,
    name: &str,
) -> GeomResult<Axis> {
    let degree = usize::from(degree);
    let start = knots[degree];
    let end = knots[count];
    if start >= end {
        return Err(invalid(&format!(
            "periodic surface {name} domain must be finite and positive"
        )));
    }
    let unique_count = if periodic {
        let period = exact_scalar_subtract(end, start).ok_or_else(|| {
            invalid(&format!(
                "periodic surface {name} period is not an exact binary64 difference"
            ))
        })?;
        let unique = count
            .checked_sub(degree)
            .ok_or_else(|| invalid(&format!("periodic surface {name} has no unique controls")))?;
        if unique <= degree {
            return Err(invalid(&format!(
                "periodic surface {name} requires more unique controls than its degree"
            )));
        }
        let extension = degree
            .checked_mul(2)
            .ok_or_else(|| invalid(&format!("periodic surface {name} degree overflows usize")))?;
        for index in 0..=extension {
            let shifted = unique
                .checked_add(index)
                .ok_or_else(|| invalid(&format!("periodic surface {name} knot index overflows")))?;
            if exact_scalar_subtract(knots[shifted], knots[index]) != Some(period)
                || exact_scalar_add(knots[index], period) != Some(knots[shifted])
            {
                return Err(invalid(&format!(
                    "periodic surface {name} knot extension is not an exact binary64 translation"
                )));
            }
        }
        for offset in 0..degree {
            let prefix_source = unique
                .checked_sub(degree)
                .and_then(|index| index.checked_add(offset))
                .ok_or_else(|| {
                    invalid(&format!("periodic surface {name} prefix index overflows"))
                })?;
            let suffix_source = degree
                .checked_mul(2)
                .and_then(|index| index.checked_add(1))
                .and_then(|index| index.checked_add(offset))
                .ok_or_else(|| {
                    invalid(&format!("periodic surface {name} suffix index overflows"))
                })?;
            if exact_scalar_subtract(knots[prefix_source], period).is_none()
                || exact_scalar_add(knots[suffix_source], period).is_none()
            {
                return Err(invalid(&format!(
                    "periodic surface {name} outer knot extension is not exact in binary64"
                )));
            }
        }
        unique
    } else {
        count
    };
    let seam_continuity_order =
        if periodic {
            let seam_multiplicity = knots.iter().filter(|&&knot| knot == start).count();
            let continuity = degree.checked_sub(seam_multiplicity).ok_or_else(|| {
                invalid(&format!(
                    "periodic surface {name} seam multiplicity exceeds degree"
                ))
            })?;
            Some(u16::try_from(continuity).map_err(|_| {
                invalid(&format!("periodic surface {name} continuity overflows u16"))
            })?)
        } else {
            None
        };
    Ok(Axis {
        domain: (start, end),
        degree,
        unique_count,
        periodic,
        seam_continuity_order,
    })
}

fn validate_aliases(surface: &BSplineSurface, u: Axis, v: Axis) -> GeomResult<()> {
    if u.periodic {
        for offset in 0..u.degree {
            let duplicate = u.unique_count + offset;
            if surface.control_points[offset] != surface.control_points[duplicate] {
                return Err(invalid(
                    "periodic surface U control rows do not exactly repeat",
                ));
            }
            if let Some(weights) = &surface.weights {
                if weights[offset] != weights[duplicate] {
                    return Err(invalid(
                        "periodic surface U weight rows do not exactly repeat",
                    ));
                }
            }
        }
    }
    if v.periodic {
        for row in 0..surface.control_points.len() {
            for offset in 0..v.degree {
                let duplicate = v.unique_count + offset;
                if surface.control_points[row][offset] != surface.control_points[row][duplicate] {
                    return Err(invalid(
                        "periodic surface V control columns do not exactly repeat",
                    ));
                }
                if let Some(weights) = &surface.weights {
                    if weights[row][offset] != weights[row][duplicate] {
                        return Err(invalid(
                            "periodic surface V weight columns do not exactly repeat",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn aliases(index: usize, axis: Axis, name: &str) -> GeomResult<Vec<usize>> {
    if index >= axis.unique_count {
        return Err(invalid(&format!(
            "periodic surface {name} control index is outside the unique net"
        )));
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(2)
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "periodic surface control aliases",
        })?;
    result.push(index);
    if axis.periodic && index < axis.degree {
        result.push(axis.unique_count + index);
    }
    Ok(result)
}

fn wrap_axis(parameter: Scalar, axis: Axis) -> GeomResult<Scalar> {
    if !parameter.is_finite() {
        return Err(invalid("periodic surface parameter must be finite"));
    }
    if !axis.periodic {
        return Ok(parameter);
    }
    let (start, end) = axis.domain;
    if parameter >= start && parameter < end {
        return Ok(parameter);
    }
    let period = exact_scalar_subtract(end, start)
        .ok_or_else(|| invalid("periodic surface period is not exact in binary64"))?;
    let offset = parameter - start;
    if !offset.is_finite() {
        return Err(invalid(
            "periodic surface parameter offset exceeds finite arithmetic",
        ));
    }
    let spacing = binary64_spacing(offset).ok_or_else(|| {
        invalid("periodic surface parameter offset has no finite binary64 spacing")
    })?;
    if spacing >= period {
        return Err(invalid(
            "periodic surface parameter offset cannot resolve one period",
        ));
    }
    let wrapped = start + offset.rem_euclid(period);
    if !wrapped.is_finite() || wrapped < start || wrapped >= end {
        return Err(invalid("periodic surface parameter could not be wrapped"));
    }
    Ok(wrapped)
}

fn binary64_spacing(value: Scalar) -> Option<Scalar> {
    let magnitude = value.abs();
    let next_bits = magnitude.to_bits().checked_add(1)?;
    let next = Scalar::from_bits(next_bits);
    let spacing = next - magnitude;
    (spacing.is_finite() && spacing > 0.0).then_some(spacing)
}

fn midpoint((start, end): (Scalar, Scalar)) -> GeomResult<Scalar> {
    let midpoint = start + (end - start) * 0.5;
    if midpoint.is_finite() {
        Ok(midpoint)
    } else {
        Err(invalid("periodic surface domain midpoint is non-finite"))
    }
}

pub(crate) fn exact_scalar_add(left: Scalar, right: Scalar) -> Option<Scalar> {
    let sum = left + right;
    if !sum.is_finite() {
        return None;
    }
    // Knuth TwoSum: `error` is the exact residual of the rounded binary64 sum.
    let right_virtual = sum - left;
    let error = (left - (sum - right_virtual)) + (right - right_virtual);
    (error == 0.0).then_some(sum)
}

pub(crate) fn exact_scalar_subtract(left: Scalar, right: Scalar) -> Option<Scalar> {
    exact_scalar_add(left, -right)
}

fn invalid(message: &str) -> GeomError {
    GeomError::InvalidInput(message.to_owned())
}
