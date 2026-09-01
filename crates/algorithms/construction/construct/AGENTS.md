# `axiolid-construct`

Scalar geometry-construction algorithms over neutral representations. This is **L2**:
it accepts exact profiles, curves, primitives, certified NURBS traces, and explicit
policy; it creates the current `TriMesh` reference result plus focused analytic B-rep
arrangements; it owns no DAG, cache, execution context, or operation-provider dispatch.

## Entry points

- `profile`: lower profile values to sampled rings and triangulate caps.
- `extrude`, `revolve`, `sweep`, `loft`: place/stitch station rings into solids.
- `center_line`: turn constant-width centre-line profiles into rings.
- `half_space`: construct a finite clipping proxy for an unbounded half-space.
- `trimmed_intersection`: promote one certified affine trace into two closed trimmed
  faces on its boundary-owned patch and an explicit embedded pcurve on the containing
  unsplit face; see ADR 0029.

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
- Discrete sweeps remain the broad reference path. Exact output is currently limited
  to the certified affine trimmed-intersection arrangement. Do not generalize that
  slice to exact sweeps, booleans, dual-boundary ownership, corners, or curved traces;
  see ADR 0020, ADR 0023, ADR 0024, and ADR 0029.

## Tests

Unit-like generation tests live in `tests/` here. Tests that verify a generated
mesh is accepted by an L3 Boolean provider stay in `axiolid-compile/tests/`:
letting this L2 crate dev-depend on that provider violates the tier boundary.
