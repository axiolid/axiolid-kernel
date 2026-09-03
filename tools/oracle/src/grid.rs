//! Deterministic parameter-box sampling.
//!
//! Sampling is the oracle's only search strategy. It is intentionally dumb: a
//! uniform closed grid with both endpoints included, so a result claimed at a
//! box corner is always visited and a reviewer can reason about coverage
//! without reading an adaptive heuristic.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::Scalar;

/// A closed parameter interval to scan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterSpan {
    /// Inclusive lower parameter.
    pub start: Scalar,
    /// Inclusive upper parameter.
    pub end: Scalar,
}

impl ParameterSpan {
    /// Construct a finite, correctly ordered span.
    pub fn new(start: Scalar, end: Scalar) -> GeomResult<Self> {
        if !start.is_finite() || !end.is_finite() || end < start {
            return Err(GeomError::InvalidInput(
                "oracle parameter span must be finite and non-inverted".to_owned(),
            ));
        }
        Ok(Self { start, end })
    }

    /// Construct a degenerate span pinned to one parameter value.
    pub fn at(value: Scalar) -> GeomResult<Self> {
        Self::new(value, value)
    }

    /// Sample position `index` of `density`, always hitting both endpoints.
    fn sample(self, index: u32, density: SampleDensity) -> Scalar {
        let steps = density.steps();
        if steps == 0 || self.start == self.end {
            return self.start;
        }
        if index >= steps {
            return self.end;
        }
        let ratio = Scalar::from(index) / Scalar::from(steps);
        // Endpoint-exact interpolation: at ratio 0 and 1 this returns the
        // stored bounds bit-for-bit rather than a rounded midpoint blend.
        self.start + (self.end - self.start) * ratio
    }
}

/// How finely a span is scanned.
///
/// This is an explicit budget, not a hidden default: an oracle that silently
/// samples too coarsely reports false agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleDensity(u32);

impl SampleDensity {
    /// Construct a density of `steps` subdivisions per span.
    ///
    /// A span is visited at `steps + 1` positions including both endpoints.
    pub fn new(steps: u32) -> GeomResult<Self> {
        if steps == 0 || steps > 4096 {
            return Err(GeomError::InvalidInput(
                "oracle sample density must be in 1..=4096 steps".to_owned(),
            ));
        }
        Ok(Self(steps))
    }

    /// Subdivisions per span.
    pub const fn steps(self) -> u32 {
        self.0
    }

    /// Number of visited positions per span.
    pub const fn positions(self) -> u32 {
        self.0 + 1
    }
}

/// Visit every sample of one span.
pub(crate) fn scan(span: ParameterSpan, density: SampleDensity) -> impl Iterator<Item = Scalar> {
    (0..density.positions()).map(move |index| span.sample(index, density))
}
