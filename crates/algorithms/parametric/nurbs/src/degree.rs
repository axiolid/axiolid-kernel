//! Knot removal and degree operations (#33).
//!
//! # Which of these are exact, and which are not
//!
//! Degree ELEVATION is exact: every degree-p B-spline is also a degree-(p+1)
//! B-spline, and the elevated control net represents the same curve. Nothing
//! is approximated, so it always succeeds on valid input.
//!
//! Knot REMOVAL and degree REDUCTION are not. A knot is removable only if the
//! curve is smooth enough there to be represented without it; a degree is
//! reducible only if the curve was already representable at the lower degree.
//! Neither is generally true, so both must either meet a stated tolerance or
//! REFUSE.
//!
//! That is the whole design constraint here. An implementation that always
//! returns something has silently approximated, and the caller cannot tell a
//! clean removal from a lossy one. These return the deviation they actually
//! introduced so the caller can check it against their own budget.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{BSplineCurve, BSplineCurve2, BSplineCurve3};
use axiolid_evaluate::curve::{bspline_jet2, bspline_jet3};

/// Outcome of a lossy operation: the curve, and the error it introduced.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundedResult<C> {
    /// The transformed curve.
    pub curve: C,
    /// Upper bound on the deviation from the original, in model units.
    ///
    /// Zero means the operation was exact. A caller comparing this against
    /// its own budget is the intended use; the operation does not decide
    /// whether the error is acceptable.
    pub deviation_upper_bound: Scalar,
}

/// Raise a planar curve's degree by one without changing its image.
///
/// Exact: a degree-p curve is also a degree-(p+1) curve. Rational curves
/// elevate in homogeneous coordinates, so weights are carried through rather
/// than discarded.
pub fn elevate_degree2(curve: &BSplineCurve2) -> GeomResult<BSplineCurve2> {
    bspline_jet2(curve, 0.0)?;
    let segments = crate::transform::bezier_segments2(curve)?;
    let extract = |p: &Point2| [p.x, p.y];
    let rebuild = |c: [Scalar; 2]| Point2::new(c[0], c[1]);
    assemble(curve, &segments, extract, rebuild)
}

/// Raise a spatial curve's degree by one without changing its image.
pub fn elevate_degree3(curve: &BSplineCurve3) -> GeomResult<BSplineCurve3> {
    bspline_jet3(curve, 0.0)?;
    let segments = crate::transform::bezier_segments3(curve)?;
    let extract = |p: &Point3| [p.x, p.y, p.z];
    let rebuild = |c: [Scalar; 3]| Point3::new(c[0], c[1], c[2]);
    assemble(curve, &segments, extract, rebuild)
}

/// Degree elevation for a single clamped Bezier segment, in homogeneous form.
///
/// The standard identity: the elevated control points are the convex
/// combination `(i/(p+1)) * P[i-1] + (1 - i/(p+1)) * P[i]`, with the endpoints
/// carried through unchanged. Exact for polynomial and rational alike, because
/// it is applied to homogeneous coordinates.
fn elevate_bezier<const N: usize>(
    points: &[[Scalar; N]],
    weights: &[Scalar],
) -> (Vec<[Scalar; N]>, Vec<Scalar>) {
    let p = points.len() - 1;
    let elevated = p + 2;
    let mut out_points = Vec::with_capacity(elevated);
    let mut out_weights = Vec::with_capacity(elevated);

    out_points.push(points[0]);
    out_weights.push(weights[0]);
    for i in 1..=p {
        let alpha = i as Scalar / (p as Scalar + 1.0);
        let mut coordinate = [0.0; N];
        for (axis, value) in coordinate.iter_mut().enumerate() {
            // Homogeneous blend: weight the position by its own weight first.
            *value = alpha * points[i - 1][axis] * weights[i - 1]
                + (1.0 - alpha) * points[i][axis] * weights[i];
        }
        let weight = alpha * weights[i - 1] + (1.0 - alpha) * weights[i];
        for value in &mut coordinate {
            *value /= weight;
        }
        out_points.push(coordinate);
        out_weights.push(weight);
    }
    out_points.push(points[p]);
    out_weights.push(weights[p]);
    (out_points, out_weights)
}

/// Assemble elevated Bezier segments into one clamped curve.
///
/// Each segment elevates exactly; joining them keeps Bezier form, so every
/// internal knot carries multiplicity `q` for elevated degree `q`. The curve
/// is identical to the input. The representation is deliberately not
/// knot-minimal: collapsing the redundant internal knots is a lossy step
/// (`remove_knot`), and folding it in here would hide an approximation inside
/// an operation documented as exact.
fn assemble<const N: usize, P: Clone>(
    curve: &BSplineCurve<P>,
    segments: &[BSplineCurve<P>],
    coordinates: impl Fn(&P) -> [Scalar; N],
    point: impl Fn([Scalar; N]) -> P,
) -> GeomResult<BSplineCurve<P>> {
    let elevated_degree = curve.degree.checked_add(1).ok_or_else(|| {
        GeomError::InvalidInput("degree elevation would overflow the degree".to_owned())
    })?;

    let mut control_points: Vec<P> = Vec::new();
    let mut weights: Vec<Scalar> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let points: Vec<[Scalar; N]> = segment.control_points.iter().map(&coordinates).collect();
        let segment_weights = segment
            .weights
            .clone()
            .unwrap_or_else(|| vec![1.0; points.len()]);
        let (elevated_points, elevated_weights) = elevate_bezier(&points, &segment_weights);

        // Adjacent segments share their join point; keep one copy.
        let skip = usize::from(index > 0);
        for (coordinate, weight) in elevated_points.into_iter().zip(elevated_weights).skip(skip) {
            control_points.push(point(coordinate));
            weights.push(weight);
        }
    }

    // Each Bezier segment is clamped, so its first and last knot values are
    // its domain ends. Boundary knots get multiplicity q+1 (clamped), internal
    // joins get q, which is Bezier form at the elevated degree.
    let clamped = u32::from(elevated_degree) + 1;
    let internal = u32::from(elevated_degree);
    let mut knots: Vec<Scalar> = Vec::with_capacity(segments.len() + 1);
    let mut multiplicities: Vec<u32> = Vec::with_capacity(segments.len() + 1);

    for (index, segment) in segments.iter().enumerate() {
        let first = *segment
            .knots
            .first()
            .ok_or_else(|| GeomError::InvalidInput("bezier segment has no knots".to_owned()))?;
        if index == 0 {
            knots.push(first);
            multiplicities.push(clamped);
        }
        let last = *segment
            .knots
            .last()
            .ok_or_else(|| GeomError::InvalidInput("bezier segment has no knots".to_owned()))?;
        knots.push(last);
        multiplicities.push(if index + 1 == segments.len() {
            clamped
        } else {
            internal
        });
    }

    // Preserve rationality: a polynomial input stays polynomial rather than
    // acquiring a vector of ones, which would change the representation
    // without changing the curve.
    let weights = curve.weights.as_ref().map(|_| weights);

    Ok(BSplineCurve {
        degree: elevated_degree,
        control_points,
        knots,
        multiplicities,
        weights,
        knot_spec: curve.knot_spec,
        closed: curve.closed,
        self_intersect: curve.self_intersect,
    })
}
