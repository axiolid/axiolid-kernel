//! Portable mesh-boolean provider contract.

use axiolid_contracts::{
    Backend, CancellationGranularity, ExecutionOptions, GeomResult, ScratchRequirement,
};
use axiolid_core::BooleanOperator;
use axiolid_mesh::TriMesh;
use axiolid_mesh_contracts::SolidRequirements;

use crate::{BooleanEvidence, BooleanOutcome};

/// Mesh boolean provider.
///
/// Implementing this trait is the capability declaration. Providers that do not
/// implement mesh booleans must not implement this trait.
pub trait MeshBoolean: Backend {
    /// Scratch this provider needs beyond its inputs and result.
    ///
    /// Callers budget against this before dispatch. Defaults to
    /// [`ScratchRequirement::Unbounded`] so an unaudited provider is treated as
    /// unbudgetable rather than silently assumed cheap.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// How finely this provider polls a cancellation token.
    ///
    /// Defaults to [`CancellationGranularity::None`]: a provider that has not
    /// declared otherwise is assumed not to poll. Claiming responsiveness a
    /// provider does not have is worse than admitting none.
    fn cancellation_granularity(&self) -> CancellationGranularity {
        CancellationGranularity::None
    }

    /// Admissibility this provider requires of its operands.
    ///
    /// Advisory only: the registry validates at the contract level before
    /// dispatch. A provider declaring a *lower* level does not thereby get to
    /// accept looser input, and one declaring a higher level is rejected by the
    /// conformance suite for narrowing the contract.
    fn solid_requirements(&self) -> SolidRequirements {
        SolidRequirements::Oriented
    }

    /// Apply one regularized set operation.
    ///
    /// Operands are pre-validated by the registry. Returns a
    /// [`BooleanOutcome`]: the mesh plus what was done to produce it. An empty
    /// result mesh is a legitimate value, not an error.
    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome>;

    /// Subtract many tools in one batch so implementations can union or schedule
    /// cutters efficiently. The default is correct but deliberately simple.
    ///
    /// The default polls cancellation between tools, which is why the default
    /// granularity for an overriding provider must be declared honestly.
    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
        let mut evidence = BooleanEvidence {
            subject_triangles: subject.triangle_count(),
            tool_triangles: tools.iter().map(TriMesh::triangle_count).sum(),
            output_triangles: subject.triangle_count(),
            output_components: 1,
            ..BooleanEvidence::default()
        };
        let mut result = subject.clone();
        for tool in tools {
            options.check_cancelled()?;
            let outcome = self.boolean(&result, tool, BooleanOperator::Difference, options)?;
            evidence.absorb(outcome.evidence);
            result = outcome.mesh;
        }
        Ok(BooleanOutcome::new(result, evidence))
    }
}

/// Compose `A △ B` as `(A ∪ B) \ (A ∩ B)`.
///
/// Free-standing rather than a trait default so a provider cannot accidentally
/// inherit a composed implementation while reporting `sub_operations: 1`. A
/// native implementor overrides [`MeshBoolean::boolean`] and never calls this.
///
/// Composition is the reason `BooleanEvidence::sub_operations` exists: without
/// it a caller cannot tell a three-pass emulation from a single-pass primitive,
/// and the two have materially different numerical behaviour.
pub fn symmetric_difference_via_composition<P>(
    provider: &P,
    subject: &TriMesh,
    tool: &TriMesh,
    options: &ExecutionOptions,
) -> GeomResult<BooleanOutcome>
where
    P: MeshBoolean + ?Sized,
{
    options.check_cancelled()?;
    let union = provider.boolean(subject, tool, BooleanOperator::Union, options)?;
    options.check_cancelled()?;
    let intersection = provider.boolean(subject, tool, BooleanOperator::Intersection, options)?;

    // A ∩ B empty means the operands are disjoint, so A △ B == A ∪ B. Skipping
    // the final difference is not just an optimisation: subtracting an empty
    // solid is a degenerate operand many backends reject.
    if intersection.mesh.indices.is_empty() {
        let mut evidence = union.evidence;
        evidence.sub_operations = 2;
        evidence.coincident_faces_encountered |= intersection.evidence.coincident_faces_encountered;
        return Ok(BooleanOutcome::new(union.mesh, evidence));
    }

    options.check_cancelled()?;
    let difference = provider.boolean(
        &union.mesh,
        &intersection.mesh,
        BooleanOperator::Difference,
        options,
    )?;

    let mut evidence = difference.evidence;
    evidence.subject_triangles = subject.triangle_count();
    evidence.tool_triangles = tool.triangle_count();
    evidence.sub_operations = 3;
    evidence.coincident_faces_encountered |= union.evidence.coincident_faces_encountered
        || intersection.evidence.coincident_faces_encountered;
    Ok(BooleanOutcome::new(difference.mesh, evidence))
}
