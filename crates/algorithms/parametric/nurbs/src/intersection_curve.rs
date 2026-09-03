//! Constructed intersection curves from certified traces (#6).
//!
//! # What is constructed, and why it is sound
//!
//! `intersect_surface_surface_certified` proves WHERE an intersection is:
//! it returns endpoint enclosures and a transversality bound. It does not
//! return a curve. Exact booleans, section curves, offsets, and fillets all
//! need the curve itself, which is why this is the gate in front of them.
//!
//! The construction here is deliberately narrow, and its narrowness is the
//! proof. A trace is only turned into a curve when BOTH surfaces are exact
//! single-span AFFINE patches, which the trace certificate already
//! establishes. Two planes meet in a straight line, so a degree-1 segment
//! between the two certified endpoints is the EXACT intersection, not a
//! sampled approximation of it:
//!
//! - each affine patch is a subset of a plane, and the signed distance to a
//!   plane is an AFFINE function of position;
//! - an affine function on a segment attains its extremes at the endpoints;
//! - so a deviation bound proven at both endpoints bounds the WHOLE segment.
//!
//! That is why the reported `deviation_upper_bound` is a real bound over the
//! curve rather than a bound at the two points that happened to be checked.
//!
//! Everything else REFUSES by name. A curved or unresolved trace has no
//! straight-line proof, and emitting a polyline there would be a tessellation
//! wearing the word "certified".

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar};
use axiolid_curve::{BSplineCurve3, KnotSpec};
use axiolid_surface::BSplineSurface;

use crate::certified_curve_surface_intersection::{
    intersect_curve_surface_certified, CertifiedCurveSurfaceIntersection3,
    CertifiedCurveSurfaceIntersectionOptions,
};
use crate::certified_surface_surface_intersection::{
    intersect_surface_surface_certified, CertifiedSurfaceSurfaceIntersection3,
    CertifiedSurfaceSurfaceIntersectionOptions, TransverseSurfaceSurfaceTrace3,
};

/// Why a certified query produced no constructed intersection curve.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum IntersectionCurveRefusal {
    /// The underlying certified query did not resolve every candidate.
    ///
    /// Constructing a curve from an incomplete cover would assert an extent
    /// the search never proved.
    Unresolved {
        /// Number of conservative candidate boxes still outstanding.
        candidates: usize,
    },
    /// The inputs are not the exact affine family this construction proves.
    ///
    /// A curved intersection has no straight-line proof; emitting a polyline
    /// would be tessellation, not certification.
    UnsupportedGeometry,
    /// The surfaces provably do not meet, so there is no curve to construct.
    Disjoint,
}

/// An intersection curve constructed from a certified trace.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructedIntersectionCurve3 {
    /// The constructed curve in model space.
    ///
    /// Exact for the affine family: two planes meet in a straight line, so
    /// this degree-1 curve IS the intersection, not a sampling of it.
    pub curve: BSplineCurve3,
    /// Upper bound on how far the curve departs from either surface.
    ///
    /// Valid over the WHOLE curve, not just the endpoints: distance to a plane
    /// is affine, so its extremes on a segment are attained at the endpoints.
    pub deviation_upper_bound: Scalar,
    /// Transversality bound carried over from the certified trace.
    pub normal_cross_squared_lower_bound: Scalar,
    /// The trace this curve was constructed from.
    pub trace: TransverseSurfaceSurfaceTrace3,
}

/// Construct intersection curves for two clamped NURBS surfaces.
///
/// Returns `Ok(Ok(curves))` only when every candidate was resolved AND every
/// retained trace admits an exact straight-line construction.
///
/// A structurally sound refusal is `Ok(Err(..))`: "these surfaces do not meet"
/// and "this shape is not provable here" are answers about geometry. `Err` is
/// reserved for invalid input and exhausted budgets.
pub fn construct_surface_surface_curves(
    first: &BSplineSurface,
    second: &BSplineSurface,
    options: CertifiedSurfaceSurfaceIntersectionOptions,
) -> GeomResult<Result<Vec<ConstructedIntersectionCurve3>, IntersectionCurveRefusal>> {
    let traces = match intersect_surface_surface_certified(first, second, options)? {
        CertifiedSurfaceSurfaceIntersection3::Complete { traces, .. } => traces,
        // Partial knowledge must not become a confident curve.
        CertifiedSurfaceSurfaceIntersection3::Unresolved {
            candidate_boxes, ..
        } => {
            return Ok(Err(IntersectionCurveRefusal::Unresolved {
                candidates: candidate_boxes.len(),
            }));
        }
    };

    // A resolved query with no trace is a PROOF of disjointness, which is a
    // different fact from "we could not tell" and is named separately.
    if traces.is_empty() {
        return Ok(Err(IntersectionCurveRefusal::Disjoint));
    }

    let mut curves = Vec::new();
    curves
        .try_reserve_exact(traces.len())
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "constructed intersection curve allocation",
        })?;
    for trace in traces {
        // Every trace must be constructible. Returning the provable subset and
        // dropping the rest would silently under-report the intersection.
        let Some(curve) = construct_from_trace(&trace) else {
            return Ok(Err(IntersectionCurveRefusal::UnsupportedGeometry));
        };
        curves.push(curve);
    }
    Ok(Ok(curves))
}

/// Build the exact segment between a trace endpoints.
///
/// Returns `None` when the endpoints coincide: a zero-length "curve" would be
/// a point contact reported as a segment, which downstream booleans would
/// mistake for an edge.
fn construct_from_trace(
    trace: &TransverseSurfaceSurfaceTrace3,
) -> Option<ConstructedIntersectionCurve3> {
    let start = trace.start.point;
    let end = trace.end.point;
    if !start.is_finite() || !end.is_finite() || start == end {
        return None;
    }

    // Affine distance-to-plane means the endpoint residuals bound the segment.
    let deviation_upper_bound = trace
        .start
        .residual_upper_bound
        .max(trace.end.residual_upper_bound);
    if !deviation_upper_bound.is_finite() {
        return None;
    }

    Some(ConstructedIntersectionCurve3 {
        curve: segment(start, end),
        deviation_upper_bound,
        normal_cross_squared_lower_bound: trace.normal_cross_squared_lower_bound,
        trace: trace.clone(),
    })
}

fn segment(start: Point3, end: Point3) -> BSplineCurve3 {
    BSplineCurve3 {
        degree: 1,
        control_points: vec![start, end],
        knots: vec![0.0, 1.0],
        multiplicities: vec![2, 2],
        weights: None,
        knot_spec: KnotSpec::PiecewiseBezier,
        closed: false,
        self_intersect: Some(false),
    }
}

/// A certified curve/surface crossing, reported as a point rather than a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstructedCurveSurfacePoint3 {
    /// Native curve parameter of the crossing.
    pub curve_parameter: Scalar,
    /// Native surface parameters of the crossing.
    pub surface_parameters: [Scalar; 2],
    /// Model-space position of the crossing.
    pub point: Point3,
}

/// Construct curve/surface intersection points for a clamped NURBS pair.
///
/// A transverse curve/surface intersection is ISOLATED: the correct output is
/// a set of points, and manufacturing a curve through them would invent an
/// extent nothing proved. A curve arises only when the curve LIES IN the
/// surface, which is a coincident case this certification does not cover and
/// which is refused rather than approximated.
pub fn construct_curve_surface_points(
    curve: &BSplineCurve3,
    surface: &BSplineSurface,
    options: CertifiedCurveSurfaceIntersectionOptions,
) -> GeomResult<Result<Vec<ConstructedCurveSurfacePoint3>, IntersectionCurveRefusal>> {
    let intersections = match intersect_curve_surface_certified(curve, surface, options)? {
        CertifiedCurveSurfaceIntersection3::Complete { intersections, .. } => intersections,
        CertifiedCurveSurfaceIntersection3::Unresolved {
            candidate_boxes, ..
        } => {
            return Ok(Err(IntersectionCurveRefusal::Unresolved {
                candidates: candidate_boxes.len(),
            }));
        }
    };
    if intersections.is_empty() {
        return Ok(Err(IntersectionCurveRefusal::Disjoint));
    }

    let mut points = Vec::new();
    points
        .try_reserve_exact(intersections.len())
        .map_err(|_| GeomError::BudgetExceeded {
            resource: "constructed intersection point allocation",
        })?;
    for intersection in intersections {
        points.push(ConstructedCurveSurfacePoint3 {
            curve_parameter: midpoint(intersection.curve_parameter),
            surface_parameters: [
                midpoint(intersection.surface_u_parameter),
                midpoint(intersection.surface_v_parameter),
            ],
            point: intersection.point,
        });
    }
    Ok(Ok(points))
}

fn midpoint(interval: crate::certified_projection::ParameterInterval) -> Scalar {
    interval.start * 0.5 + interval.end * 0.5
}
