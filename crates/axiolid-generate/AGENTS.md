# `axiolid-generate`

Scalar solid-generation algorithms over neutral representations. This is **L2**:
it accepts exact profiles, curves, primitives, and explicit tolerance policy; it
creates the current `TriMesh` reference result; it owns no DAG, cache, execution
context, or operation-provider dispatch.

## Entry points

- `profile`: lower profile values to sampled rings and triangulate caps.
- `extrude`, `revolve`, `sweep`, `loft`: place/stitch station rings into solids.
- `center_line`: turn constant-width centre-line profiles into rings.
- `half_space`: construct a finite clipping proxy for an unbounded half-space.

`BACKEND_ID` is `scalar-generate`. Use it for every diagnostic raised here; do
not report `scalar-compile` after this split.

## Invariants

- Every tolerance-sensitive entry point receives an explicit `Tolerance` and/or
  chord budget. Never introduce a global default epsilon.
- Profile mismatch, unbounded construction, invalid frames, insufficient rings,
  and unmet subdivision budgets refuse with `GeomError`; never invent a mesh.
- Shared loft/stitching logic owns winding and cap pairing. Do not duplicate it
  in individual sweep families.
- This crate must not depend on `axiolid-model`, an execution/backend crate, or
  any L3 crate. The executable layering test is
  `axiolid-core/tests/layering.rs`.
- Generation is deliberately the current **discrete** reference path. Do not
  claim it preserves exact B-rep output. An eventual exact generator belongs in
  this layer, not in `axiolid-compile`; see ADR 0020 and ADR 0023.

## Tests

Unit-like generation tests live in `tests/` here. Tests that verify a generated
mesh is accepted by an L3 Boolean provider stay in `axiolid-compile/tests/`:
letting this L2 crate dev-depend on that provider violates the tier boundary.
