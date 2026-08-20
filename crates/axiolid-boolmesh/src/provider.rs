//! The `MeshBoolean` implementation.

use axiolid_core::BooleanOperator;
use axiolid_kernel::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, MeshBoolean, ScratchRequirement,
};
use axiolid_mesh::TriMesh;
use boolmesh::prelude::{compute_boolean, OpType};

use crate::convert::{from_manifold, six_signed_volume, to_manifold};

/// Mesh boolean backed by `boolmesh` (pure Rust, `glam`-only, MPL-2.0).
///
/// Adopted rather than written: see `docs/adr/0014`. This type owns the
/// conversion and contract enforcement; the algorithm itself is upstream's.
#[derive(Debug, Clone, Copy, Default)]
pub struct BoolmeshBoolean;

impl BoolmeshBoolean {
    /// Stable identifier used in errors and explicit provider selection.
    pub const ID: BackendId = BackendId::new("boolmesh");

    /// Construct the provider.
    pub const fn new() -> Self {
        Self
    }
}

impl Backend for BoolmeshBoolean {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(Self::ID, ExecutionTarget::PortableCpu)
    }
}

/// Verify the result against the contract before handing it back.
///
/// `boolmesh` reports its own manifoldness; we additionally check orientation,
/// because a returned inside-out solid would poison every subsequent operation
/// in a subtract chain. Blamed on the backend, not the caller: the inputs were
/// already validated on the way in.
fn check_result(result: &TriMesh, operation: BooleanOperator) -> GeomResult<()> {
    // An empty result is legitimate: subtracting a tool that fully contains the
    // subject leaves nothing. Orientation is undefined for it, so stop here.
    //
    // Removing this early return is behaviour-preserving (an empty mesh sums to
    // zero, not a negative), so no test can distinguish it. It is kept because
    // it states the intent at the point of the decision.
    if result.indices.is_empty() {
        return Ok(());
    }
    let six_volume = six_signed_volume(&result.positions, &result.indices);
    if six_volume < 0.0 {
        return Err(GeomError::BackendContractViolation {
            backend: BoolmeshBoolean::ID,
            detail: format!(
                "{operation:?} returned an inside-out solid (signed volume {:.6})",
                six_volume / 6.0
            ),
        });
    }
    Ok(())
}

impl MeshBoolean for BoolmeshBoolean {
    /// `boolmesh` builds a Morton collider and intersection tables sized by the
    /// combined input, and does not expose a bound. Declaring `Unbounded` keeps
    /// it honest: a caller with a hard memory budget will be refused rather
    /// than silently allowed to allocate past it.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::Unbounded
    }

    /// Group mutually disjoint cutters and remove each group with one
    /// boolean, instead of one boolean per cutter.
    ///
    /// Rests on `(S \ A) \ B == S \ (A union B)` and on a concatenation of
    /// disjoint solids being their union. Bounding-box grouping over-separates
    /// but never wrongly fuses, so the result is identical to the sequential
    /// default -- gated by volume equality in `tests/batch.rs`.
    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.subtract_grouped(subject, tools, options)
    }

    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        _options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let op = match operation {
            BooleanOperator::Union => OpType::Add,
            BooleanOperator::Intersection => OpType::Intersect,
            BooleanOperator::Difference => OpType::Subtract,
        };
        let subject_manifold = to_manifold(subject, "subject")?;
        let tool_manifold = to_manifold(tool, "tool")?;

        let output = compute_boolean(&subject_manifold, &tool_manifold, op).map_err(|reason| {
            GeomError::Degenerate(format!("boolmesh {operation:?} failed: {reason}"))
        })?;

        let result = from_manifold(&output);
        check_result(&result, operation)?;
        Ok(result)
    }
}

/// Batch difference: fuse mutually disjoint cutters, then subtract per group.
///
/// The sequential default runs N booleans, each against a subject that has
/// already been cut N-1 times. This override runs one boolean per GROUP of
/// mutually disjoint cutters, which for the dominant layout (a wall with
/// non-overlapping openings) collapses to a single boolean.
///
/// Correctness rests on `(S \ A) \ B == S \ (A union B)`, plus the fact that a
/// concatenation of disjoint solids IS their union. Grouping uses bounding
/// boxes, which over-separate but never wrongly fuse, so the result is
/// identical to the sequential path -- asserted by the volume gates.
impl BoolmeshBoolean {
    /// Subtract every tool, grouping disjoint ones into single operations.
    pub(crate) fn subtract_grouped(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let bounds: Vec<_> = tools.iter().map(TriMesh::bounds).collect();
        let groups = crate::grouping::disjoint_groups(&bounds);

        let mut current = subject.clone();
        for group in &groups {
            // A single-member group gains nothing from fusing, so skip the
            // copy and subtract the tool directly.
            if let [only] = group.as_slice() {
                current = self.difference(&current, &tools[*only], options)?;
                continue;
            }
            let members: Vec<&TriMesh> = group.iter().map(|&i| &tools[i]).collect();
            let fused = crate::grouping::fuse(&members);
            current = self.difference(&current, &fused, options)?;
        }
        Ok(current)
    }

    /// One difference, routed through the same validation as `boolean`.
    fn difference(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.boolean(subject, tool, BooleanOperator::Difference, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiolid_core::Point3;

    /// A tetrahedron with reversed winding: the shape a faulty backend would
    /// return if it inverted its output.
    fn inside_out_tetrahedron() -> TriMesh {
        let positions = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        // Verified: signed volume -1/6, i.e. inward-facing normals.
        let indices = vec![0, 1, 2, 0, 3, 1, 0, 2, 3, 1, 3, 2];
        TriMesh::new(positions, indices)
    }

    /// `check_result` guards against an upstream regression that inverts its
    /// output. With validated inputs the current `boolmesh` release never does
    /// this -- verified by instrumenting the branch across the whole suite and
    /// observing zero hits -- so the guard is exercised directly here rather
    /// than left as untested defensive code.
    #[test]
    fn an_inside_out_result_is_blamed_on_the_backend() {
        let error = check_result(&inside_out_tetrahedron(), BooleanOperator::Difference)
            .expect_err("an inside-out result must be rejected");
        match error {
            GeomError::BackendContractViolation { backend, detail } => {
                assert_eq!(backend, BoolmeshBoolean::ID);
                assert!(detail.contains("inside-out"), "{detail}");
            }
            other => panic!("must blame the backend, not the caller: {other:?}"),
        }
    }

    /// An empty result is legitimate (tool fully contains subject) and must not
    /// be mistaken for an orientation fault.
    #[test]
    fn an_empty_result_is_accepted() {
        assert!(check_result(&TriMesh::default(), BooleanOperator::Difference).is_ok());
    }

    /// A correctly oriented result passes.
    #[test]
    fn an_outward_result_is_accepted() {
        let mut mesh = inside_out_tetrahedron();
        for corner in mesh.indices.chunks_exact_mut(3) {
            corner.swap(1, 2);
        }
        assert!(check_result(&mesh, BooleanOperator::Difference).is_ok());
    }
}
