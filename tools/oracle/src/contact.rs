//! Mapped-3D contact witnesses.
//!
//! Each helper takes a claimed parameter box, maps both operands into model
//! space through the scalar evaluator, and returns the smallest deviation it
//! observed. The deviation is the evidence: a caller asserts on the number, not
//! on a boolean the oracle decided for it.

use axiolid_contracts::GeomResult;
use axiolid_core::{Point2, Point3, Scalar};
use axiolid_curve::{Curve2, Curve3};
use axiolid_evaluate::{evaluate2, evaluate3};
use axiolid_surface::Surface;

use crate::grid::{scan, ParameterSpan, SampleDensity};
use crate::MappedDeviation;

/// Claimed parameter box for a planar or spatial curve pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePairBox {
    /// Enclosure on the first curve.
    pub first: ParameterSpan,
    /// Enclosure on the second curve.
    pub second: ParameterSpan,
}

/// Claimed parameter box for a curve against a surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSurfaceBox {
    /// Enclosure on the curve.
    pub curve: ParameterSpan,
    /// Enclosure on the surface `u` axis.
    pub surface_u: ParameterSpan,
    /// Enclosure on the surface `v` axis.
    pub surface_v: ParameterSpan,
}

/// Claimed parameter box for two surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePairBox {
    /// First-surface `u` enclosure.
    pub first_u: ParameterSpan,
    /// First-surface `v` enclosure.
    pub first_v: ParameterSpan,
    /// Second-surface `u` enclosure.
    pub second_u: ParameterSpan,
    /// Second-surface `v` enclosure.
    pub second_v: ParameterSpan,
}

/// Smallest mapped deviation between two planar curves over a claimed box.
///
/// Planar inputs are lifted to `z = 0` so every oracle verdict is expressed in
/// the same mapped 3D metric, per the milestone requirement that agreement is
/// measured in model space rather than parameter space.
pub fn curve_pair_deviation2(
    first: &Curve2,
    second: &Curve2,
    claimed: CurvePairBox,
    density: SampleDensity,
) -> GeomResult<MappedDeviation> {
    let mut best: Option<MappedDeviation> = None;
    for u in scan(claimed.first, density) {
        let a = lift(evaluate2(first, u)?);
        for v in scan(claimed.second, density) {
            let b = lift(evaluate2(second, v)?);
            keep_closest(&mut best, MappedDeviation::new(a, b));
        }
    }
    Ok(best.expect("a closed span always yields at least one sample"))
}

/// Smallest mapped deviation between two spatial curves over a claimed box.
pub fn curve_pair_deviation3(
    first: &Curve3,
    second: &Curve3,
    claimed: CurvePairBox,
    density: SampleDensity,
) -> GeomResult<MappedDeviation> {
    let mut best: Option<MappedDeviation> = None;
    for u in scan(claimed.first, density) {
        let a = evaluate3(first, u)?;
        for v in scan(claimed.second, density) {
            let b = evaluate3(second, v)?;
            keep_closest(&mut best, MappedDeviation::new(a, b));
        }
    }
    Ok(best.expect("a closed span always yields at least one sample"))
}

/// Smallest mapped deviation between a spatial curve and a surface.
pub fn curve_surface_deviation(
    curve: &Curve3,
    surface: &Surface,
    claimed: CurveSurfaceBox,
    density: SampleDensity,
) -> GeomResult<MappedDeviation> {
    let mut best: Option<MappedDeviation> = None;
    for t in scan(claimed.curve, density) {
        let a = evaluate3(curve, t)?;
        for u in scan(claimed.surface_u, density) {
            for v in scan(claimed.surface_v, density) {
                let b = axiolid_evaluate::surface::evaluate(surface, u, v)?;
                keep_closest(&mut best, MappedDeviation::new(a, b));
            }
        }
    }
    Ok(best.expect("a closed span always yields at least one sample"))
}

/// Smallest mapped deviation between two surfaces over a claimed box.
pub fn surface_pair_deviation(
    first: &Surface,
    second: &Surface,
    claimed: SurfacePairBox,
    density: SampleDensity,
) -> GeomResult<MappedDeviation> {
    let mut best: Option<MappedDeviation> = None;
    for u in scan(claimed.first_u, density) {
        for v in scan(claimed.first_v, density) {
            let a = axiolid_evaluate::surface::evaluate(first, u, v)?;
            for s in scan(claimed.second_u, density) {
                for t in scan(claimed.second_v, density) {
                    let b = axiolid_evaluate::surface::evaluate(second, s, t)?;
                    keep_closest(&mut best, MappedDeviation::new(a, b));
                }
            }
        }
    }
    Ok(best.expect("a closed span always yields at least one sample"))
}

/// Assert-friendly wrapper: the best deviation found, as a bare scalar.
pub fn contact_witness(deviation: MappedDeviation) -> Scalar {
    deviation.deviation
}

fn keep_closest(best: &mut Option<MappedDeviation>, candidate: MappedDeviation) {
    match best {
        Some(current) if current.deviation <= candidate.deviation => {}
        slot => *slot = Some(candidate),
    }
}

fn lift(point: Point2) -> Point3 {
    Point3::new(point.x, point.y, 0.0)
}
