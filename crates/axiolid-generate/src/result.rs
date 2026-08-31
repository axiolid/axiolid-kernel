//! Explicit output-model contracts for solid generation.
//!
//! Analytic B-rep and mesh tessellation are deliberately different requests and
//! result variants. A future exact generator may return `ExactBRep` only for an
//! [`GenerationRequest::ExactBRep`] request; it must return `Unsupported` rather
//! than discretising that request. Tessellation is an explicit request carrying
//! the caller's tolerance.

use axiolid_brep::ExactBRep;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;

/// Requested result model for a generation operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenerationRequest {
    /// Preserve analytic supports, trim spans, and typed topology.
    ExactBRep,
    /// Produce a triangle mesh within the stated explicit tolerance policy.
    Tessellation(TessellationRequest),
}

impl GenerationRequest {
    /// Whether this request must be satisfied by an analytic B-rep or refused.
    ///
    /// A mesh is never a valid fallback for this request.
    pub const fn requires_exact_brep(self) -> bool {
        matches!(self, Self::ExactBRep)
    }
}

/// Explicit tolerance policy for a tessellation request.
///
/// There is no default: mesh accuracy is a caller decision and must not become
/// an implicit replacement for exact construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationRequest {
    tolerance: Tolerance,
}

impl TessellationRequest {
    /// Construct an explicit tessellation request.
    pub const fn new(tolerance: Tolerance) -> Self {
        Self { tolerance }
    }

    /// Requested tolerance policy.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }
}

/// Geometry produced by a generation operation.
///
/// The variant must match the caller's [`GenerationRequest`]. This sum type is
/// intentionally not coercible: consumers must acknowledge whether they hold
/// analytic B-rep or a discrete mesh.
///
/// `ExactBRep` remains inline for public API compatibility and to avoid an
/// infallible boxing allocation on certified construction paths.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeneratedGeometry {
    /// Analytic boundary representation with explicit topology and trims.
    ExactBRep(ExactBRep),
    /// Explicitly requested triangle tessellation.
    Tessellation(TriMesh),
}

/// Coarse output-model classification for capabilities and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationOutput {
    /// Analytic B-rep result.
    ExactBRep,
    /// Triangle mesh result.
    Tessellation,
}

impl GeneratedGeometry {
    /// Result model actually produced.
    pub const fn output(&self) -> GenerationOutput {
        match self {
            Self::ExactBRep(_) => GenerationOutput::ExactBRep,
            Self::Tessellation(_) => GenerationOutput::Tessellation,
        }
    }
}
