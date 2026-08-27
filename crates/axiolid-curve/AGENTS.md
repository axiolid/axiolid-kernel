# axiolid-curve instructions

Purpose: Atomic exact curves and evaluation seams.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

linear.rs; conic.rs; spline.rs; evaluate.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Composite/trim/offset/surface relations belong in axiolid-model to avoid curve-surface cycles. Preserve knots, multiplicities, weights, and domains.

This crate is representation only: it declares `CurveEvaluator` but implements
no evaluation. The scalar implementation is `axiolid_scalar::curve`
(ADR 0018) — analytic per family, de Boor for splines, adaptive flattening on
measured sagitta. Do not add an evaluator here; L1 is data, L2 solves.

A polyline's parameter is **one unit per segment**, so a closed n-point ring
has domain `(0, n)`. A `ProfileSegment::domain` of `(0, 1)` on a multi-segment
polyline is rejected rather than silently truncated.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Tests must exercise invalid input as well as happy paths.
