# axiolid-mesh-boolean-boolmesh instructions

Purpose: adapt the adopted `boolmesh` crate to `axiolid_kernel::MeshBoolean` (ADR 0014).
This crate owns conversion and contract enforcement; the algorithm is upstream's.

## Module ownership

convert.rs (TriMesh <-> Manifold, orientation gate); provider.rs (the trait impl,
result contract). Split before unrelated concerns grow together.

## Invariants

Orientation is checked on the way IN, per argument, naming which argument failed.
An inside-out mesh is structurally valid and manifold, so nothing else catches it;
`Difference` then behaves as `Union` and returns a LARGER mesh with no error. This
happened for real during the ADR 0014 evaluation.

Input faults are `InvalidInput`/`Degenerate`/`NotManifold` (caller's fault).
Result faults are `BackendContractViolation` (upstream's fault). Never blame the
caller for an upstream defect.

Scratch is `Unbounded`: `boolmesh` exposes no bound, so a caller with a hard
budget is refused rather than silently allowed past it.

Results carry no normals. `boolmesh` computes face normals; re-exporting them as
vertex normals would misrepresent the hard edges a cut creates.

## Verification

Volume conservation (`vol(a\b) + vol(a^b) == vol(a)`) is the gate, not index
comparison: it is triangulation-invariant, so it tests geometry rather than an
output buffer we do not control. Test helpers compute volume independently of the
crate's own helper, or the test would confirm the implementation with itself.

`boolmesh` must not be re-exported. It is MPL-2.0 and swappable; leaking its types
would make the adoption visible to consumers and defeat the seam.

## Batch override

`subtract_many` groups mutually disjoint cutters (AABB overlap graph, greedy
first-fit colouring) and removes each group with one boolean. Measured 9.2x at
n=64 on the IFC-dominant layout; 0.99x worst case, so it is unconditional.

Invariants, each mutation-proven in `tests/batch.rs`:

- **Only disjoint cutters may be fused.** Concatenating overlapping solids
  yields a self-intersecting mesh; subtracting it gives a wrong answer that
  still looks like a valid result. The disjointness check is load-bearing.
- **`fuse` must rebase indices.** Forgetting the offset silently duplicates the
  first mesh's triangles.
- **Every group must be subtracted**, and the single-member fast path must use
  that group's tool, not `tools[0]`.

Volume comparisons between the grouped and sequential paths use a RELATIVE
tolerance: the two sum a differently ordered triangle list, so the last bits
legitimately differ. Bitwise equality fails spuriously.
