//! Refutation of claimed global minimum distances.
//!
//! A projection or inversion result claims that no point of the operand is
//! closer to the target than the returned bound. That claim is falsifiable: if
//! sampling finds any point strictly closer, the claim is wrong regardless of
//! how the implementation derived it.
//!
//! This module only produces refutations. Finding nothing closer is reported as
//! `None` and must never be read as a proof of global minimality.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar};
use axiolid_curve::Curve3;
use axiolid_evaluate::evaluate3;
use axiolid_surface::Surface;

use crate::grid::{scan, ParameterSpan, SampleDensity};

/// A claimed global closest-point result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceClaim {
    /// Query point the operand was projected onto.
    pub target: Point3,
    /// Claimed upper bound on the global minimum distance.
    ///
    /// A certified result reports an outward-rounded bound; the refutation
    /// scan must beat it by more than `slack` to count as a disagreement.
    pub claimed_distance: Scalar,
    /// Numerical allowance absorbed before a sample counts as strictly closer.
    pub slack: Scalar,
}

impl DistanceClaim {
    /// Construct a finite, non-negative claim.
    pub fn new(target: Point3, claimed_distance: Scalar, slack: Scalar) -> GeomResult<Self> {
        if !target.is_finite()
            || !claimed_distance.is_finite()
            || claimed_distance < 0.0
            || !slack.is_finite()
            || slack < 0.0
        {
            return Err(GeomError::InvalidInput(
                "distance claim must be finite with non-negative distance and slack".to_owned(),
            ));
        }
        Ok(Self {
            target,
            claimed_distance,
            slack,
        })
    }

    fn refuted_by(self, distance: Scalar) -> bool {
        distance < self.claimed_distance - self.slack
    }
}

/// A sampled point that is strictly closer than a claimed global minimum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloserPoint {
    /// Native parameters of the counterexample.
    pub parameters: [Scalar; 2],
    /// Mapped position of the counterexample.
    pub point: Point3,
    /// Distance from the query target, which is below the claimed minimum.
    pub distance: Scalar,
    /// How far the claim overshot: `claimed - distance`.
    pub deviation: Scalar,
}

/// Search a curve or surface for a point closer than a claimed global minimum.
///
/// Returns `Some` only when a strictly closer point was actually evaluated, so
/// a hit is a sound refutation. `None` means the scan found no counterexample
/// at this density and proves nothing on its own.
pub fn closer_point_refutation(
    operand: &Operand<'_>,
    claim: DistanceClaim,
    density: SampleDensity,
) -> GeomResult<Option<CloserPoint>> {
    let mut best: Option<CloserPoint> = None;
    match operand {
        Operand::Curve { curve, span } => {
            for t in scan(*span, density) {
                let point = evaluate3(curve, t)?;
                consider(&mut best, claim, [t, 0.0], point);
            }
        }
        Operand::Surface { surface, u, v } => {
            for su in scan(*u, density) {
                for sv in scan(*v, density) {
                    let point = axiolid_evaluate::surface::evaluate(surface, su, sv)?;
                    consider(&mut best, claim, [su, sv], point);
                }
            }
        }
    }
    Ok(best)
}

/// What a distance claim was made about.
#[derive(Debug, Clone, Copy)]
pub enum Operand<'a> {
    /// A spatial curve scanned over one parameter span.
    Curve {
        /// Curve under test.
        curve: &'a Curve3,
        /// Span to scan.
        span: ParameterSpan,
    },
    /// A surface scanned over a rectangular native domain.
    Surface {
        /// Surface under test.
        surface: &'a Surface,
        /// `u` span to scan.
        u: ParameterSpan,
        /// `v` span to scan.
        v: ParameterSpan,
    },
}

fn consider(
    best: &mut Option<CloserPoint>,
    claim: DistanceClaim,
    parameters: [Scalar; 2],
    point: Point3,
) {
    let distance = (point - claim.target).length();
    if !claim.refuted_by(distance) {
        return;
    }
    let candidate = CloserPoint {
        parameters,
        point,
        distance,
        deviation: claim.claimed_distance - distance,
    };
    match best {
        Some(current) if current.distance <= candidate.distance => {}
        slot => *slot = Some(candidate),
    }
}
