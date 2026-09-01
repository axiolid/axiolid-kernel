//! The `MeshBoolean` implementation.

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, CancellationGranularity, ExecutionOptions,
    ExecutionTarget, GeomError, GeomResult, ScratchRequirement,
};
use axiolid_core::BooleanOperator;
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_contract::{
    symmetric_difference_via_composition, BooleanEvidence, BooleanOutcome, MeshBoolean,
};
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
    if !six_volume.is_finite() {
        return Err(GeomError::BackendContractViolation {
            backend: BoolmeshBoolean::ID,
            detail: format!("{operation:?} returned non-finite signed volume"),
        });
    }
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
    /// Measured, not guessed.
    ///
    /// `boolmesh` builds a Morton collider and intersection tables sized by the
    /// combined input. It exposes no bound, so this was measured directly with
    /// a counting global allocator (`src/bin/scratch_probe.rs`) across all four
    /// operations at 24 to 1,536 input triangles:
    ///
    /// ```text
    ///  triangles   peak bytes   bytes/triangle   worst operation
    ///         24        63852             2660   SymmetricDifference
    ///         96       125580             1308   SymmetricDifference
    ///        384       438844             1142   SymmetricDifference
    ///       1536      1756060             1143   SymmetricDifference
    /// ```
    ///
    /// Consumption is linear in input size, converging to roughly 1.1 KiB per
    /// triangle; the higher ratio at small inputs is fixed setup cost being
    /// divided by few triangles. `SymmetricDifference` is worst because it is
    /// composed from three passes and holds intermediates alive.
    ///
    /// The declared bound is 4 KiB per triangle: above the worst observed
    /// ratio with roughly 1.5x headroom for allocator variance and future
    /// operand shapes. A declared bound that is occasionally too low is worse
    /// than `Unbounded`, because it makes a budget look enforced when it is not.
    fn scratch_requirement(&self) -> ScratchRequirement {
        ScratchRequirement::PerElement {
            bytes_per_element: 4096,
        }
    }

    /// Group mutually disjoint cutters and remove each group with one
    /// boolean, instead of one boolean per cutter.
    ///
    /// Rests on `(S \ A) \ B == S \ (A union B)` and on a concatenation of
    /// disjoint solids being their union. Bounding-box grouping over-separates
    /// but never wrongly fuses, so the result is identical to the sequential
    /// default -- gated by volume equality in `tests/batch.rs`.
    /// `boolmesh` takes no cancellation handle, so nothing can interrupt a
    /// single boolean once it starts. Declared honestly: the batch override
    /// polls between groups, which is the only real poll point available.
    fn cancellation_granularity(&self) -> CancellationGranularity {
        CancellationGranularity::BetweenOperations
    }

    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
        self.subtract_grouped(subject, tools, options)
    }

    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
        // `SymmetricDifference` has no `boolmesh` counterpart. Compose it from
        // the three primitives rather than pretending the backend's operation
        // set is the contract's; `sub_operations` in the evidence records that
        // the caller got a composed result.
        if operation == BooleanOperator::SymmetricDifference {
            return symmetric_difference_via_composition(self, subject, tool, options);
        }

        let op = match operation {
            BooleanOperator::Union => OpType::Add,
            BooleanOperator::Intersection => OpType::Intersect,
            BooleanOperator::Difference => OpType::Subtract,
            BooleanOperator::SymmetricDifference => unreachable!("composed above"),
            // A future operand added to the contract. Refusing is honest and
            // lets the registry fall back to a provider that supports it.
            _ => {
                return Err(GeomError::Unsupported {
                    backend: BoolmeshBoolean::ID,
                    operation: axiolid_contracts::Operation::MeshBoolean,
                })
            }
        };
        options.check_cancelled()?;
        let subject_manifold = to_manifold(subject, "subject")?;
        let tool_manifold = to_manifold(tool, "tool")?;

        let output = match compute_boolean(&subject_manifold, &tool_manifold, op) {
            Ok(output) => output,
            // An empty result is a legitimate value under the contract: the
            // intersection of disjoint solids is nothing. `boolmesh` signals
            // that by failing to build a mesh from an empty vertex matrix, so
            // translate it back into the empty solid the contract specifies
            // rather than surfacing a backend quirk as a geometry error.
            Err(reason) if is_empty_result(&reason) => {
                let empty = TriMesh::new(Vec::new(), Vec::new());
                let evidence = evidence_for(subject, &[tool], &empty, 1);
                return Ok(BooleanOutcome::new(empty, evidence));
            }
            Err(reason) => {
                return Err(GeomError::Degenerate(format!(
                    "boolmesh {operation:?} failed: {reason}"
                )))
            }
        };

        let result = from_manifold(&output);
        check_result(&result, operation)?;
        let evidence = evidence_for(subject, &[tool], &result, 1);
        Ok(BooleanOutcome::new(result, evidence))
    }
}

/// Whether a `boolmesh` failure actually means "the result is empty".
///
/// Matched on the message because the backend does not expose a typed empty
/// case. Narrow on purpose: any other failure stays a real error rather than
/// being silently converted into an empty solid, which would turn a genuine
/// fault into a plausible-looking zero-volume answer.
fn is_empty_result(reason: &impl std::fmt::Display) -> bool {
    let text = reason.to_string().to_lowercase();
    text.contains("empty pos matrix") || text.contains("empty mesh")
}

/// Build evidence for one completed operation.
///
/// `output_components` counts connected components by union-find over shared
/// vertex indices. A difference that severs a wall reports two components, so
/// the caller learns about the split here rather than in a later quantity.
fn evidence_for(
    subject: &TriMesh,
    tools: &[&TriMesh],
    result: &TriMesh,
    sub_operations: usize,
) -> BooleanEvidence {
    let disjoint = tools
        .iter()
        .filter(|tool| !subject.bounds().intersects(&tool.bounds()))
        .count();
    BooleanEvidence::record(
        subject.triangle_count(),
        tools.iter().map(|t| t.triangle_count()).sum(),
        result.triangle_count(),
        connected_components(result),
    )
    .with_disjoint_tools(disjoint)
    .with_sub_operations(sub_operations)
    // `boolmesh` does not report coincident-face encounters, so claiming
    // detection would be a lie. Left false until a provider can answer.
    .with_coincident_faces(false)
}

/// Connected components over vertices shared by triangles, via union-find.
fn connected_components(mesh: &TriMesh) -> usize {
    let count = mesh.positions.len();
    if count == 0 {
        return 0;
    }
    let mut parent: Vec<usize> = (0..count).collect();

    fn find(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }

    for triangle in mesh.indices.chunks_exact(3) {
        let a = find(&mut parent, triangle[0] as usize);
        for corner in &triangle[1..] {
            let b = find(&mut parent, *corner as usize);
            if a != b {
                parent[b] = a;
            }
        }
    }

    // Only vertices actually referenced by a triangle count: an unreferenced
    // position is not a component, it is unused data.
    let mut roots = std::collections::BTreeSet::new();
    for index in &mesh.indices {
        let root = find(&mut parent, *index as usize);
        roots.insert(root);
    }
    roots.len()
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
    ) -> GeomResult<BooleanOutcome> {
        let bounds: Vec<_> = tools.iter().map(TriMesh::bounds).collect();
        let groups = crate::grouping::disjoint_groups(&bounds);

        let mut current = subject.clone();
        let mut sub_operations = 0;
        for group in &groups {
            // The only real poll point: between groups. Cancelling here returns
            // no mesh at all rather than a partially cut one.
            options.check_cancelled()?;
            sub_operations += 1;
            // A single-member group gains nothing from fusing, so skip the
            // copy and subtract the tool directly.
            if let [only] = group.as_slice() {
                current = self.difference(&current, &tools[*only], options)?.mesh;
                continue;
            }
            let members: Vec<&TriMesh> = group.iter().map(|&i| &tools[i]).collect();
            let fused = crate::grouping::fuse(&members);
            current = self.difference(&current, &fused, options)?.mesh;
        }
        let borrowed: Vec<&TriMesh> = tools.iter().collect();
        let evidence = evidence_for(subject, &borrowed, &current, sub_operations);
        Ok(BooleanOutcome::new(current, evidence))
    }

    /// One difference, routed through the same validation as `boolean`.
    fn difference(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        options: &ExecutionOptions,
    ) -> GeomResult<BooleanOutcome> {
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

    fn positive_infinite_volume() -> TriMesh {
        let large = 1.0e308;
        let small = 1.0e-308;
        TriMesh::new(
            vec![
                Point3::new(small, small, 1.0),
                Point3::new(large, 1.0, small),
                Point3::new(small, large, 1.0),
            ],
            vec![0, 1, 2],
        )
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

    #[test]
    fn a_non_finite_result_is_blamed_on_the_backend() {
        let error = check_result(&positive_infinite_volume(), BooleanOperator::Union)
            .expect_err("a non-finite result volume must be rejected");
        assert!(
            matches!(error, GeomError::BackendContractViolation { backend, ref detail }
                if backend == BoolmeshBoolean::ID && detail.contains("non-finite signed volume")),
            "must blame the backend, got {error:?}"
        );
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
