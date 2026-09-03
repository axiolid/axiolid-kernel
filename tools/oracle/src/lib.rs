#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Independent mapped-3D verification oracle for intersection and inversion.
//!
//! # Why this crate exists
//!
//! Parameter-space agreement is not evidence of geometric correctness. Two
//! implementations that share an assumption agree on the same wrong answer, and
//! a certified parameter box that is never mapped into model space proves
//! nothing a downstream consumer can use.
//!
//! This crate maps a claimed result back into 3D through the portable scalar
//! evaluator and measures the deviation there. It deliberately does **not**
//! depend on `axiolid-nurbs`, so it shares no subdivision, isolation, or
//! interval machinery with the implementations it checks.
//!
//! # What it can and cannot do
//!
//! The oracle is a **falsifier**, not a prover:
//!
//! - [`contact_witness`] and friends *search* a parameter box and return the
//!   best agreement they found. A small deviation is a witness that the box
//!   really does contain near-coincident geometry. A large deviation means the
//!   sampling did not find one, which is not by itself a disproof.
//! - [`closer_point_refutation`] scans for a point strictly closer than a
//!   claimed global minimum. A hit is a **sound refutation**: the claim is
//!   wrong. No hit is not a proof of global minimality.
//!
//! Every helper reports the 3D deviation it measured, so a failing test can say
//! how far off the result was rather than only that it disagreed.

mod contact;
mod distance;
mod grid;

pub use contact::{
    contact_witness, curve_pair_deviation2, curve_pair_deviation3, curve_surface_deviation,
    surface_pair_deviation, CurvePairBox, CurveSurfaceBox, SurfacePairBox,
};
pub use distance::{closer_point_refutation, CloserPoint, DistanceClaim, Operand};
pub use grid::{ParameterSpan, SampleDensity};

use axiolid_core::{Point3, Scalar};

/// A measured disagreement between two mapped 3D positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MappedDeviation {
    /// Euclidean distance between the two mapped points.
    pub deviation: Scalar,
    /// Point produced by the first operand.
    pub first: Point3,
    /// Point produced by the second operand.
    pub second: Point3,
}

impl MappedDeviation {
    fn new(first: Point3, second: Point3) -> Self {
        Self {
            deviation: (second - first).length(),
            first,
            second,
        }
    }
}
