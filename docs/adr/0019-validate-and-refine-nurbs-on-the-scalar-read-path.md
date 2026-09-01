# 0019 — Validate and refine NURBS on the scalar read path

- **Status:** Accepted
- **Date:** 2026-08-30
- **Deciders:** Friedrich, axiolid
- **Relates to:** 0012 (scalar reference ownership), 0015 (Earcut), 0018 (curve evaluation)

## Context

Axiolid could represent and evaluate polynomial and rational B-spline curves and
surfaces, but three gaps made the imported-data path unsafe to describe as a
reliable CAD reader foundation:

1. Compact knot values and multiplicities were expanded with `zip`. Mismatched
   lengths were silently truncated, while a hostile multiplicity could allocate
   before the expected expanded size was checked.
2. Rational B-spline surface normals used parameter-scaled finite differences.
   This loses analytic information and can collapse to a zero step when CAD
   parameter origins are large.
3. Curved B-rep compilation sampled pcurve boundaries and sent only those points
   to Earcut. The vertices lay on the support surface, but long interior
   diagonals could miss it by far more than the caller's chord budget.
4. Refining each Earcut triangle independently could split only one side of a
   shared edge. A mesh-index-only midpoint cache did not propagate that split
   and could alias unrelated edges on different faces.
5. Adaptive curve flattening could return success at recursion exhaustion or
   floating-point midpoint collapse without having met its tolerance.
6. Whole curved bounds did not apply `FaceBound.orientation`, and the first
   stored bound was assumed to be outer even when another bound was explicitly
   marked outer. Finite source values could also overflow in homogeneous space,
   while NaN chord errors compared as if they met tolerance.
7. Per-edge and per-face limits did not bound cumulative tessellation-owned work.

The required behavior is a safe, format-neutral read and discretization path.
It is not NURBS authoring and does not imply a complete CAD modeling kernel.

## Decision

**The scalar reference validates compact spline data before allocation, computes
rational surface partials analytically, and refines curved trim triangles against
their support surface under explicit budgets.**

Concretely:

- `axiolid-scalar` uses one crate-private `SplineAxis` validator for curve and
  surface axes. Distinct-knot/multiplicity lengths, finite and strictly
  increasing knots, non-zero bounded multiplicities, exact expanded size,
  control dimensions, finite controls, and finite positive weights are checked
  before expansion or evaluation. Derived homogeneous products, recurrence
  outputs, Cartesian projections, and elementary frames must also remain finite.
- Rational tensor-product surface point and first-partial evaluation happens in
  homogeneous coordinates. Cartesian partials use the quotient rule; normals
  use `normalize(∂S/∂u × ∂S/∂v)` rather than finite differences.
- Curved B-rep edges cache complete endpoint-inclusive sample sequences. A
  reversed edge use reverses the complete sequence, while face loops omit the
  final endpoint. Shared pcurve images must agree in 3D within tolerance. Curved
  bounds select the explicit outer ring, apply whole-bound orientation to paired
  UV/index sequences, and propagate outer orientation to emitted triangle winding.
  Missing, duplicate, or undersampled outer rings fail closed.
- A hole-free axis-aligned rectangular trim with uniform parameter spacing is
  seeded as a structured grid so no initial diagonal spans distant surface
  cells. Grid recognition declines on holes, slanted/nonuniform trims, malformed
  cyclic perimeter order, duplicate samples, or conflicting boundary topology
  and falls back to Earcut. Both seed paths then
  enter the same face-level refiner: failing interior-edge split requests are
  collected first and applied to every incident triangle; centroid-only
  failures use a conforming centroid fan. Sampled pcurve boundary segments are
  not replaced by affine parameter edges.
- Refinement vertices and midpoint keys use face-local identities distinct from
  welded mesh indices. Midpoints are reused only by triangles incident to the
  same face-local edge. Cross-face sharing remains the responsibility of the
  topological-edge sample cache.
- Elementary periodic surfaces unwrap complete trim rings into one continuous
  face chart before Earcut. Hole charts are aligned to the outer chart, and all
  descendants retain that chart rather than choosing triangle-local antipodal
  branches.
- Boundary sampling fails closed after `4096` segments per topological edge.
  Each curved face may process at most `262144` face-local vertex records,
  counting boundary occurrences and created refinement vertices; refinement also
  fails closed after depth `20`. Across one tessellation, curved-face records and
  emitted positions are each capped at `4194304`, and emitted indices at
  `16777216`; planar batches check remaining capacity before mutation, while a
  curved face rolls back all output, cache, and aggregate-accounting changes on
  any error. Input preflight caps faces at `65536`; each
  topology table and the
  aggregate edge-use, face-bound, and shell-face reference sets, plus expanded
  outer-shell trim work, are capped at `1048576`. Pre-existing shell vertices do not consume a later face's per-face
  allowance, while reused boundary occurrences still do.
- Adaptive curve flattening fails closed if depth is exhausted above tolerance
  or if a finite parameter interval can no longer be bisected.
  A missing budget or inconsistent shared pcurve produces a structured error,
  not an unchecked approximation.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep finite-difference normals | Large parameter origins can round the step to zero; the homogeneous derivative is already available analytically. |
| Expand knots and validate afterward | Allows silent `zip` truncation and attacker-controlled allocation before rejection. |
| Trust boundary vertices plus Earcut | Proves only that vertices lie on the support; it does not bound support-surface error along interior diagonals. |
| Add centroid-only triangle fans | Leaves original Earcut diagonals intact, so their error cannot converge. |
| Refine each triangle independently | Creates T-junctions and cracks along shared triangle and face edges. |
| Introduce a CAD kernel dependency | Violates the pure-Rust, provider-neutral layering and is unnecessary for this bounded reference operation. |
| Claim general periodic B-spline support from `u_closed`/`v_closed` | Those flags preserve source metadata; periodic spline evaluator semantics are not established by this change. |

## Consequences

**Positive**

- Malformed imported spline data fails deterministically before large allocation
  or silent geometric reinterpretation.
- Rational B-spline normals remain valid at large knot origins and are backed by
  exact first partials.
- Trimmed rational B-spline faces now have executable support-surface chord-error
  evidence. Structured rectangular grids prevent long cross-patch seed
  diagonals; guarded Earcut fallback preserves holes and irregular trims.
  Analytic-cylinder fixtures cover conforming asymmetric refinement, strict
  periodic closure and area, cross-face seam reuse, and pcurve-hole preservation.
- The scalar path remains pure Rust and format neutral.

**Negative / costs**

- Previously tolerated invalid inputs, including zero/negative rational weights,
  now return `InvalidInput`.
- Support-surface refinement performs multiple scalar surface evaluations per
  triangle and can grow the mesh substantially for tight budgets. No performance
  claim is made.
- The midpoint/centroid probe is a bounded tessellation criterion, not a formal
  global curvature proof between every possible parameter point.

**Follow-ups / risks to watch**

- Add differential surface-partial fixtures independent of the implementation
  for repeated-knot and one-sided derivative behavior.
- Add an adjacent rational-B-spline-face fixture; current adjacent-edge
  conformity and cross-face seam fixtures use analytic cylinders.
- Define periodic B-spline evaluation only with a tested knot/control convention.
- NURBS intersections, closest-point inversion, knot editing, degree operations,
  splitting, and Bézier decomposition remain unsupported.

## Relation to existing code

- `crates/algorithms/reference/src/nurbs.rs`
- `crates/algorithms/reference/src/curve.rs`
- `crates/algorithms/reference/src/surface.rs`
- `crates/execution/compile/src/brep.rs`
- `crates/algorithms/reference/tests/{curve,surface}.rs`
- `crates/execution/compile/tests/brep_tessellation.rs`
