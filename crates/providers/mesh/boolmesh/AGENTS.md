# axiolid-mesh-boolean-boolmesh instructions

Purpose: adapt the adopted `boolmesh` crate to `axiolid_contracts::MeshBoolean` (ADR 0014).
This crate owns conversion and contract enforcement; the algorithm is upstream's.

## Module ownership

convert.rs (TriMesh <-> Manifold, orientation gate); provider.rs (the trait impl,
result contract); box_detect.rs (axis-aligned box recognition); cellular.rs (the
analytic subtraction construction). Split before unrelated concerns grow together.

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

## Analytic box path (opt-in)

`subtract_boxes_analytic` cuts axis-aligned boxes out of an axis-aligned box in
closed form. ~25x faster than the general solver at n=64 openings.

**Opt-in, never auto-dispatched.** Unlike the batch override above (unconditional
because its worst case is 0.99x), this path changes the OUTPUT TOPOLOGY, not just
the schedule. Dispatching on shape would make triangle counts depend on whether a
wall's openings happened to be axis-aligned. The caller asks, and handles
`Ok(None)`.

Invariants, each mutation-proven in `tests/analytic_boxes.rs`:

- **Recognition is structural, never by bounding box.** Every mesh has a bounding
  box; a sphere and its enclosing cube share one. Acceptance requires an exact
  index count, all corners on the min/max lattice, and exactly 2 triangles per
  face plane.
- **The index-count check is not redundant with the plane check.** `chunks_exact(3)`
  silently drops a trailing partial triangle, so a malformed 38-index buffer
  presents a perfect box to the plane loop. Only the length check sees it.
- **The lattice check is not redundant either**, for one reason: it walks ALL
  positions, while the plane check only sees REFERENCED ones. An unused
  off-lattice vertex is invisible to the latter.
- **Refusal must stay a refusal.** Returning a wrong solid is worse than
  returning nothing; every decline case has a test.

Three independent oracles are needed, because each is blind to a different
defect:

- signed volume misses cancelling errors (an inverted face pair sums to zero);
- edge pairing misses coincident duplicate faces (each edge still balances);
- duplicate-face detection is the only one that catches an emitted interior face.

The interior-face mutant produced 96 triangles instead of 64 with IDENTICAL
volume and ZERO edge-pairing defects. Volume alone would have passed it.
