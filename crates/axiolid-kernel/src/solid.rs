//! Solid admissibility: what a mesh must satisfy to be a boolean operand.
//!
//! # Why this lives in the kernel
//!
//! Before this module, the only provider validated its own inputs inside its
//! adapter. That made the first backend the de-facto definition of "valid
//! solid": a second provider would have brought a second, silently different
//! definition, and callers would have seen admissibility change when dispatch
//! picked a different backend.
//!
//! Axiolid owns admissibility. The registry validates *before* dispatch, so a
//! provider never sees an operand the contract rejects. Providers must not
//! widen the set (accepting what Axiolid rejects) or narrow it (rejecting what
//! Axiolid accepts); the conformance suite checks both directions.

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;

use crate::{GeomError, GeomResult};

/// How strictly an operand must be formed.
///
/// Levels are cumulative: each includes the checks below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SolidRequirements {
    /// Indices in range, no NaN/infinite coordinates, at least one triangle.
    ///
    /// The floor. An operand failing this is malformed data, not geometry.
    Structural,
    /// Structural, plus a finite, non-zero enclosed signed volume.
    ///
    /// Volume accumulation that overflows or otherwise becomes non-finite is
    /// rejected rather than treated as evidence of a valid interior.
    ///
    /// A flat or self-cancelling shell has no interior, so no set operation on
    /// it has a defined meaning.
    Enclosing,
    /// Enclosing, plus outward orientation (positive signed volume).
    ///
    /// An inside-out operand would silently invert the operation, turning a
    /// difference into an intersection without any error.
    Oriented,
}

/// Why an operand was rejected, with the operand named.
///
/// `role` is `"subject"` or `"tool[3]"` so a caller learns *which* mesh was
/// wrong, not merely that some mesh was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidRejection {
    /// Which operand failed.
    pub role: String,
    /// The level it failed at.
    pub level: SolidRequirements,
    /// Human-readable detail.
    pub detail: String,
}

impl SolidRequirements {
    /// Validate one operand at this level.
    pub fn validate(self, mesh: &TriMesh, role: &str) -> GeomResult<()> {
        mesh.validate_structure()
            .map_err(|error| GeomError::InvalidInput(format!("{role}: {error}")))?;
        if mesh.indices.is_empty() {
            return Err(GeomError::InvalidInput(format!(
                "{role}: mesh has no triangles"
            )));
        }
        if !mesh
            .positions
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite() && point.z.is_finite())
        {
            return Err(GeomError::InvalidInput(format!(
                "{role}: mesh has a non-finite coordinate"
            )));
        }
        if self == Self::Structural {
            return Ok(());
        }

        let six_volume = six_signed_volume(mesh);
        if !six_volume.is_finite() {
            return Err(GeomError::InvalidInput(format!(
                "{role}: mesh has non-finite signed volume"
            )));
        }
        if six_volume == 0.0 {
            return Err(GeomError::Degenerate(format!(
                "{role}: mesh encloses zero signed volume, so it has no interior"
            )));
        }
        if self == Self::Enclosing {
            return Ok(());
        }

        if six_volume < 0.0 {
            return Err(GeomError::InvalidInput(format!(
                "{role}: mesh is inside-out (signed volume {:.6} < 0); \
                 boolean operations would silently invert",
                six_volume / 6.0
            )));
        }
        Ok(())
    }

    /// Validate a subject and its tools, naming each operand by index.
    pub fn validate_operands(self, subject: &TriMesh, tools: &[&TriMesh]) -> GeomResult<()> {
        self.validate(subject, "subject")?;
        for (index, tool) in tools.iter().enumerate() {
            self.validate(tool, &format!("tool[{index}]"))?;
        }
        Ok(())
    }
}

/// Six times the signed volume, via the divergence theorem about the origin.
///
/// The factor of six is left in deliberately: the sign and the zero test are
/// what matter, and dividing introduces rounding for no benefit.
pub(crate) fn six_signed_volume(mesh: &TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|triangle| {
            let a = mesh.positions[triangle[0] as usize];
            let b = mesh.positions[triangle[1] as usize];
            let c = mesh.positions[triangle[2] as usize];
            a.dot(b.cross(c))
        })
        .sum()
}

/// Enclosed volume of a validated operand, used by conformance invariants.
pub fn enclosed_volume(mesh: &TriMesh, _tolerance: Tolerance) -> f64 {
    six_signed_volume(mesh) / 6.0
}
