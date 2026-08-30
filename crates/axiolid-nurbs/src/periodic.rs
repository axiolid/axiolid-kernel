//! Verified periodic parameter semantics for closed NURBS curves.

use crate::axis::active_spans;
use axiolid_core::{Scalar, Tolerance};
use axiolid_curve::{BSplineCurve2, BSplineCurve3};
use axiolid_kernel::{GeomError, GeomResult};
use axiolid_scalar::curve::{bspline_jet2, bspline_jet3, CurveJet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
/// Highest endpoint continuity verified in the curve's native parameter.
pub enum SeamContinuity {
    /// Endpoint positions differ beyond tolerance.
    Discontinuous,
    /// Endpoint positions agree, but first derivatives do not.
    Position,
    /// Positions and first derivatives agree, but second derivatives do not.
    FirstDerivative,
    /// Positions and first two derivatives agree.
    SecondDerivative,
}

/// Verify native-parameter endpoint continuity of a planar B-spline curve.
pub fn curve2_seam_continuity(
    curve: &BSplineCurve2,
    tolerance: Tolerance,
) -> GeomResult<SeamContinuity> {
    let (lo, hi) = domain(
        curve.degree,
        curve.control_points.len(),
        &curve.knots,
        &curve.multiplicities,
    )?;
    let a = bspline_jet2(curve, lo)?;
    let b = bspline_jet2(curve, hi)?;
    Ok(classify(a, b, tolerance))
}

/// Verify native-parameter endpoint continuity of a spatial B-spline curve.
pub fn curve3_seam_continuity(
    curve: &BSplineCurve3,
    tolerance: Tolerance,
) -> GeomResult<SeamContinuity> {
    let (lo, hi) = domain(
        curve.degree,
        curve.control_points.len(),
        &curve.knots,
        &curve.multiplicities,
    )?;
    let a = bspline_jet3(curve, lo)?;
    let b = bspline_jet3(curve, hi)?;
    Ok(classify(a, b, tolerance))
}

/// Wrap a finite planar-curve parameter into its active domain.
///
/// Wrapping is rejected unless `closed` is set and position continuity is
/// independently verified. An in-domain upper endpoint remains the upper
/// endpoint rather than being remapped to the lower endpoint.
pub fn wrap_curve2_parameter(
    curve: &BSplineCurve2,
    parameter: Scalar,
    tolerance: Tolerance,
) -> GeomResult<Scalar> {
    if !curve.closed || curve2_seam_continuity(curve, tolerance)? < SeamContinuity::Position {
        return Err(GeomError::InvalidInput(
            "curve is not a verified closed seam".to_owned(),
        ));
    }
    wrap(
        parameter,
        domain(
            curve.degree,
            curve.control_points.len(),
            &curve.knots,
            &curve.multiplicities,
        )?,
    )
}

/// Wrap a finite spatial-curve parameter into its active domain.
///
/// The same verified-closed precondition as [`wrap_curve2_parameter`] applies.
pub fn wrap_curve3_parameter(
    curve: &BSplineCurve3,
    parameter: Scalar,
    tolerance: Tolerance,
) -> GeomResult<Scalar> {
    if !curve.closed || curve3_seam_continuity(curve, tolerance)? < SeamContinuity::Position {
        return Err(GeomError::InvalidInput(
            "curve is not a verified closed seam".to_owned(),
        ));
    }
    wrap(
        parameter,
        domain(
            curve.degree,
            curve.control_points.len(),
            &curve.knots,
            &curve.multiplicities,
        )?,
    )
}

fn classify<P, D>(a: CurveJet<P, D>, b: CurveJet<P, D>, tolerance: Tolerance) -> SeamContinuity
where
    P: Copy + core::ops::Sub<P, Output = D>,
    D: Copy + core::ops::Sub<D, Output = D> + Length,
{
    if (a.point - b.point).length() > tolerance.linear() {
        return SeamContinuity::Discontinuous;
    }
    if !close_vector(a.first, b.first, tolerance) {
        return SeamContinuity::Position;
    }
    if !close_vector(a.second, b.second, tolerance) {
        return SeamContinuity::FirstDerivative;
    }
    SeamContinuity::SecondDerivative
}

fn close_vector<D: Copy + core::ops::Sub<D, Output = D> + Length>(
    a: D,
    b: D,
    tolerance: Tolerance,
) -> bool {
    let scale = a.length().max(b.length()).max(1.0);
    (a - b).length() <= tolerance.linear() + tolerance.angular() * scale
}

trait Length {
    fn length(self) -> Scalar;
}
impl Length for axiolid_core::Vec2 {
    fn length(self) -> Scalar {
        self.length()
    }
}
impl Length for axiolid_core::Vec3 {
    fn length(self) -> Scalar {
        self.length()
    }
}

fn domain(
    degree: u16,
    count: usize,
    knots: &[Scalar],
    multiplicities: &[u32],
) -> GeomResult<(Scalar, Scalar)> {
    let spans = active_spans(knots, multiplicities, degree, count)?;
    Ok((spans[0].0, spans[spans.len() - 1].1))
}

fn wrap(parameter: Scalar, (lo, hi): (Scalar, Scalar)) -> GeomResult<Scalar> {
    if !parameter.is_finite() {
        return Err(GeomError::InvalidInput(
            "parameter must be finite".to_owned(),
        ));
    }
    if parameter >= lo && parameter <= hi {
        return Ok(parameter);
    }
    Ok(lo + (parameter - lo).rem_euclid(hi - lo))
}
