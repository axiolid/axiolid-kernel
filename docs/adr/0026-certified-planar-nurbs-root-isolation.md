# 0026 — Planar NURBS roots require interval existence proofs

- **Status:** Accepted
- **Date:** 2026-08-31
- **Deciders:** Axiolid maintainers
- **Supersedes:** —

## Context

A zero or near-zero curve-pair distance does not prove an intersection. Newton
convergence likewise proves neither existence nor uniqueness, and can miss
other roots. A complete planar result must account for every pair of clamped
parameter spans while refusing singular, coincident, or resource-exhausted
cases it cannot classify.

## Decision

`axiolid-nurbs` owns a pure-Rust, outward-rounded planar root-isolation oracle
for the residual

`F(u, v) = C₁(u) - C₂(v)`.

- Both curves are validated and refined to positive-weight rational Bézier
  cells under one shared pre-allocation refinement budget from ADR 0025.
  Policies are capped at 100,000 generated nodes and depth 64; oversized caller
  budgets are invalid. Initial span pairs are traversed one at a time. The
  depth-bounded DFS stack stores only base-cell indices and parameter boxes;
  restricted control nets are reconstructed for the current item and never
  retained in deferred work.
- Each cell pair is pruned only when an outward-rounded coordinate interval of
  `F` excludes zero.
- Rational native-parameter derivative intervals are derived in homogeneous
  coordinates with the quotient rule. The denominator interval must remain
  strictly positive.
- A scalar jet at the box midpoint supplies only a Krawczyk preconditioner. It
  is treated as a stored `f64` interval constant; it does not establish a root.
- A nonlinear root is accepted only when the outward-rounded Krawczyk image is
  strictly inside the source parameter box and the interval Jacobian
  determinant excludes zero. This proves one transverse root in that box.
- A proven Krawczyk image contracts the next search cell, which is re-proved
  before completion. Stored Bézier cells are not contracted below the requested
  parameter width, avoiding
  derivative-enclosure inflation from dividing ulp-scale control uncertainty by
  a smaller native span.
- A `Complete` result means every initial span pair was either excluded or
  represented by an isolated transverse result whose two native-parameter
  intervals are no wider than `CertifiedCurveIntersectionOptions::parameter_tolerance()`.
  The tolerance must be finite and strictly positive. Depth-limited or singular
  boxes produce an explicit `Unresolved` outcome. Node exhaustion remains
  `GeomError::BudgetExceeded` and cannot return a partial complete result.
- Single-span polynomial line segments additionally use Axiolid's exact-sign
  `orient2d` cascade. Zero-length degree-one curves use exact point/segment
  predicates and report a distinct zero-dimensional `PointContact`; they can
  never establish positive-dimensional overlap. Structurally identical
  non-constant curves produce `Overlap`, localized to corresponding Bézier-span
  parameter boxes only.
  A polynomial line and quadratic Bézier sharing an endpoint, tangent control
  direction, and strictly one-sided remaining control produce the narrow
  structurally proven `Tangency` outcome.

Returned scalar points are representatives. Root existence, uniqueness, and
transversality come from the parameter-box proof, not from a small reported
residual.

## Capability boundary

This decision implements bounded planar clamped curve/curve classification for:

- complete disjoint results;
- exact-sign single-span line intersections and zero-length `PointContact` cases;
- contractive transverse polynomial and positive-weight rational Bézier roots;
- the narrow structural overlap and endpoint-tangency cases above;
- explicit unresolved boxes everywhere else.

Strict-interior Krawczyk currently leaves roots on dyadic subdivision
boundaries or Bézier seams unresolved unless a structural classifier owns them.
There is not yet ownership-aware boundary deduplication, exhaustive general
tangency/coincidence classification, spatial curve intersection,
curve/surface or surface/surface intersection, tracing, pcurve construction, or
B-rep splitting. No downstream code may infer those capabilities from this API.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Treat minimum distance below tolerance as a root | Proximity does not prove existence or count roots. |
| Report converged Newton iterates | Local convergence is not exhaustive and does not classify singular roots. |
| Accept Krawczyk images that merely overlap the source box | Overlap is not an existence/uniqueness proof. |
| Merge nearby candidates by epsilon | Scale-dependent clustering can merge distinct roots or duplicate one root. |
| Hide unresolved boxes in a complete result | It would make completeness false and poison topology consumers. |

## Consequences

**Positive**

- `Complete` has an executable exhaustive meaning.
- Polynomial and rational roots share one homogeneous interval oracle.
- Singular and boundary cases remain visible rather than guessed.
- Resource amplification is bounded before refinement and during product-cell
  subdivision.

**Negative / costs**

- Wide derivative enclosures can delay or prevent contraction.
- Strict interior proof intentionally returns unresolved boundary roots.
- General tangent and overlap classification needs additional algebraic or
  interval-topological proof machinery.

## Follow-ups

1. Add ownership-aware seam/boundary proofs and overlap-based certified-box
   deduplication.
2. Add general tangent/coincident classification and parameter-reversal
   metamorphic corpora.
3. Generalize cell/derivative enclosures to curve/surface and tensor-product
   surface/surface systems.
4. Construct model-space intersection curves and paired pcurves only from
   certified seeds and deviation bounds.

## Relation to existing code

- `crates/algorithms/parametric/nurbs/src/certified_curve_intersection.rs`
- `crates/algorithms/parametric/nurbs/src/certified_bezier.rs`
- `crates/algorithms/parametric/nurbs/src/certified_refinement.rs`
- `crates/algorithms/parametric/nurbs/tests/certified_curve_intersection.rs`
